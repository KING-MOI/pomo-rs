use crate::bar;
use crate::config::{Config, Display};
use crate::duration;
use crate::keys::{self, Key};
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

struct Runner<'a, W: Write> {
    config: &'a Config,
    log_path: &'a Path,
    count: Option<u32>,
    keys: Option<&'a Receiver<Key>>,
    live: bool,
    out: W,
}

pub fn run(config: &Config, count: Option<u32>, log_path: &Path) -> io::Result<()> {
    let out = io::stdout();
    let live = out.is_terminal();
    let hide = config.display == Display::Bar && live;
    let listener = keys::listen();
    let mut r = Runner {
        config,
        log_path,
        count,
        keys: listener.as_ref().map(|(_, rx)| rx),
        live,
        out,
    };
    if hide {
        write!(r.out, "{}", bar::HIDE_CURSOR)?;
    }
    let result = r.cycle_loop();
    if hide {
        write!(r.out, "{}", bar::SHOW_CURSOR)?;
        r.out.flush()?;
    }
    drop(listener);
    result
}

impl<W: Write> Runner<'_, W> {
    fn cycle_loop(&mut self) -> io::Result<()> {
        let mut cycle = Cycle::new(self.config.long_break_every);
        loop {
            let kind = cycle.current();
            let mut timer = Timer::new(kind.length(self.config));
            let started = chrono::Local::now().fixed_offset();
            let outcome = self.session(kind, &mut timer)?;
            let actual = match outcome {
                Outcome::Finished => timer.length(),
                _ => timer.elapsed(Instant::now()).min(timer.length()),
            };
            let word = match outcome {
                Outcome::Finished => "done",
                Outcome::Skipped => "skipped",
                Outcome::Quit => "stopped",
            };
            if self.live {
                write!(self.out, "{}", bar::CLEAR_LINE)?;
            }
            writeln!(self.out, "{} {word}", kind.label())?;
            log::append(
                self.log_path,
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
                notify::send(self.config.notify, &msg)?;
            }
            if self.count.is_some_and(|n| cycle.completed_work() >= n) {
                return Ok(());
            }
        }
    }

    fn session(&mut self, kind: Kind, timer: &mut Timer) -> io::Result<Outcome> {
        timer.start(Instant::now());
        if !self.live {
            writeln!(self.out, "{} started ({})", kind.label(), duration::clock(timer.length()))?;
        }
        while !timer.is_done(Instant::now()) {
            self.draw(kind, timer)?;
            let key = match self.keys {
                Some(rx) => rx.recv_timeout(TICK).ok(),
                None => {
                    thread::sleep(TICK);
                    None
                }
            };
            match key {
                Some(Key::Toggle) => timer.toggle(Instant::now()),
                Some(Key::Extend) => timer.extend(Duration::from_secs(60)),
                Some(Key::Restart) => timer.restart(Instant::now()),
                Some(Key::Skip) => return Ok(Outcome::Skipped),
                Some(Key::Quit) => return Ok(Outcome::Quit),
                None => {}
            }
        }
        Ok(Outcome::Finished)
    }

    fn draw(&mut self, kind: Kind, timer: &Timer) -> io::Result<()> {
        if !self.live {
            return Ok(());
        }
        let now = Instant::now();
        let c = self.config;
        let line = match c.display {
            Display::Plain => bar::plain(kind, timer.remaining(now), !timer.is_running()),
            Display::Bar => bar::frame(
                kind,
                timer.progress(now),
                timer.remaining(now),
                !timer.is_running(),
                c.bar_width,
                c.color,
            ),
        };
        write!(self.out, "{}{}", bar::CLEAR_LINE, line)?;
        self.out.flush()
    }
}
