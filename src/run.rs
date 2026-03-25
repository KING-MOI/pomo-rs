use crate::config::Config;
use crate::duration;
use crate::session::Cycle;
use crate::timer::Timer;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

pub fn run(config: &Config, count: Option<u32>) -> io::Result<()> {
    let mut cycle = Cycle::new(config.long_break_every);
    let mut out = io::stdout();
    loop {
        let kind = cycle.current();
        let mut timer = Timer::new(kind.length(config));
        timer.start(Instant::now());
        while !timer.is_done(Instant::now()) {
            let left = duration::clock(timer.remaining(Instant::now()));
            write!(out, "\r{} {}", kind.label(), left)?;
            out.flush()?;
            thread::sleep(Duration::from_millis(250));
        }
        writeln!(out, "\r{} done      ", kind.label())?;
        cycle.advance();
        if count.is_some_and(|n| cycle.completed_work() >= n) {
            return Ok(());
        }
    }
}
