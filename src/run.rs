use crate::bar;
use crate::keys::{self, Key};
use crate::config::{Config, Display};
use crate::log::{self, Entry};
use crate::notify;
use crate::session::{Cycle, Kind};
use crate::timer::Timer;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Finished,
    Skipped,
    Quit,
}

pub fn run(config: &Config, count: Option<u32>, log_path: &Path) -> io::Result<()> {
    let mut cycle = Cycle::new(config.long_break_every);
    let mut out = io::stdout();
    let hide = config.display == Display::Bar && out.is_terminal();
    if hide {
        write!(out, "{}", bar::HIDE_CURSOR)?;
    }
    let listener = keys::listen();
    let rx = listener.as_ref().map(|(_, rx)| rx);
    let result = cycle_loop(config, count, log_path, rx, &mut cycle, &mut out);
    drop(listener);
    if hide {
        write!(out, "{}", bar::SHOW_CURSOR)?;
        out.flush()?;
    }
    result
}

fn cycle_loop(
    config: &Config,
    count: Option<u32>,
    log_path: &Path,
    rx: Option<&Receiver<Key>>,
    cycle: &mut Cycle,
    out: &mut impl Write,
) -> io::Result<()> {
    loop {
        let kind = cycle.current();
        let mut timer = Timer::new(kind.length(config));
        let started = chrono::Local::now().fixed_offset();
        let outcome = run_session(config, kind, &mut timer, rx, out)?;
        let actual = match outcome {
            Outcome::Finished => timer.length(),
            _ => timer.elapsed(Instant::now()).min(timer.length()),
        };
        let word = match outcome {
            Outcome::Finished => "done",
            Outcome::Skipped => "skipped",
            Outcome::Quit => "stopped",
        };
        writeln!(out, "{}{} {word}", bar::CLEAR_LINE, kind.label())?;
        log::append(
            log_path,
            &Entry {
                start: started,
                kind,
                planned: timer.length(),
                actual,
            },
        )?;
        if outcome == Outcome::Quit {
            return Ok(());
        }
        let next = cycle.advance();
        if outcome == Outcome::Finished {
            let msg = format!("{} done, time for {}", kind.label(), next.label());
            notify::send(config.notify, &msg)?;
        }
        if count.is_some_and(|n| cycle.completed_work() >= n) {
            return Ok(());
        }
    }
}

fn run_session(
    config: &Config,
    kind: Kind,
    timer: &mut Timer,
    rx: Option<&Receiver<Key>>,
    out: &mut impl Write,
) -> io::Result<Outcome> {
    timer.start(Instant::now());
    while !timer.is_done(Instant::now()) {
        draw(config, kind, timer, out)?;
        let key = match rx {
            Some(rx) => rx.recv_timeout(TICK).ok(),
            None => {
                thread::sleep(TICK);
                None
            }
        };
        match key {
            Some(Key::Toggle) => timer.toggle(Instant::now()),
            Some(Key::Skip) => return Ok(Outcome::Skipped),
            Some(Key::Quit) => return Ok(Outcome::Quit),
            None => {}
        }
    }
    Ok(Outcome::Finished)
}

fn draw(config: &Config, kind: Kind, timer: &Timer, out: &mut impl Write) -> io::Result<()> {
    let now = Instant::now();
    let line = match config.display {
        Display::Plain => bar::plain(kind, timer.remaining(now)),
        Display::Bar => bar::frame(
            kind,
            timer.progress(now),
            timer.remaining(now),
            config.bar_width,
            config.color,
        ),
    };
    write!(out, "{}{}", bar::CLEAR_LINE, line)?;
    out.flush()
}
