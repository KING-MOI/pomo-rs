use crate::bar;
use crate::config::{Config, Display};
use crate::log::{self, Entry};
use crate::notify;
use crate::session::Cycle;
use crate::timer::Timer;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

pub fn run(config: &Config, count: Option<u32>, log_path: &Path) -> io::Result<()> {
    let mut cycle = Cycle::new(config.long_break_every);
    let mut out = io::stdout();
    let hide = config.display == Display::Bar && out.is_terminal();
    if hide {
        write!(out, "{}", bar::HIDE_CURSOR)?;
    }
    let result = cycle_loop(config, count, log_path, &mut cycle, &mut out);
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
    cycle: &mut Cycle,
    out: &mut impl Write,
) -> io::Result<()> {
    loop {
        let kind = cycle.current();
        let mut timer = Timer::new(kind.length(config));
        let started = chrono::Local::now().fixed_offset();
        timer.start(Instant::now());
        while !timer.is_done(Instant::now()) {
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
            out.flush()?;
            thread::sleep(Duration::from_millis(250));
        }
        writeln!(out, "{}{} done", bar::CLEAR_LINE, kind.label())?;
        log::append(
            log_path,
            &Entry {
                start: started,
                kind,
                planned: timer.length(),
                actual: timer.length(),
            },
        )?;
        let next = cycle.advance();
        notify::send(config.notify, &format!("{} done, time for {}", kind.label(), next.label()))?;
        if count.is_some_and(|n| cycle.completed_work() >= n) {
            return Ok(());
        }
    }
}
