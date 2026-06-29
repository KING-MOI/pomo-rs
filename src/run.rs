use crate::bar;
use crate::config::{Config, Display};
use crate::hook;
use crate::duration;
use crate::keys::{self, Key};
use crate::log::{self, Entry};
use crate::notify;
use crate::remote::{self, State};
use crate::session::{Cycle, Kind};
use crate::timer::Timer;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, SystemTime};

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
    state_dir: &'a Path,
    task: Option<&'a str>,
    count: Option<u32>,
    keys: Option<&'a Receiver<Key>>,
    live: bool,
    done_today: u32,
    out: W,
}

/// Full work sessions already logged today, so the goal counter picks up
/// where an earlier run left off.
fn done_today(log_path: &Path) -> u32 {
    let today = chrono::Local::now().date_naive();
    log::read(log_path)
        .map(|(entries, _)| {
            entries
                .iter()
                .filter(|e| e.kind == Kind::Work && e.completed())
                .filter(|e| e.start.date_naive() == today)
                .count() as u32
        })
        .unwrap_or(0)
}

pub fn run(
    config: &Config,
    task: Option<&str>,
    count: Option<u32>,
    log_path: &Path,
    state_dir: &Path,
) -> io::Result<()> {
    let out = io::stdout();
    let live = out.is_terminal();
    let hide = config.display == Display::Bar && live;
    let listener = keys::listen();
    let mut r = Runner {
        config,
        log_path,
        state_dir,
        task,
        count,
        keys: listener.as_ref().map(|(_, rx)| rx),
        live,
        done_today: done_today(log_path),
        out,
    };
    if hide {
        write!(r.out, "{}", bar::HIDE_CURSOR)?;
    }
    if r.keys.is_some() && live {
        writeln!(
            r.out,
            "space pause · s skip · + minute · r restart · q quit"
        )?;
    }
    let result = r.cycle_loop();
    remote::clear(state_dir);
    if config.title && live {
        write!(r.out, "{}", bar::title(""))?;
        r.out.flush()?;
    }
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
                _ => timer.elapsed(SystemTime::now()).min(timer.length()),
            };
            let word = match outcome {
                Outcome::Finished => "done",
                Outcome::Skipped => "skipped",
                Outcome::Quit => "stopped",
            };
            if self.live {
                write!(self.out, "{}", bar::CLEAR_LINE)?;
            }
            write!(self.out, "{} {word}", kind.label())?;
            let goal = self.config.daily_goal;
            if kind == Kind::Work && outcome == Outcome::Finished {
                self.done_today += 1;
                if goal > 0 {
                    write!(self.out, " ({}/{goal} today)", self.done_today)?;
                }
            }
            writeln!(self.out)?;
            log::append(
                self.log_path,
                &Entry {
                    start: started,
                    kind,
                    planned: timer.length(),
                    actual,
                    task: self.task.map(str::to_string),
                },
            )?;
            if let Some(cmd) = &self.config.hook {
                hook::run(cmd, kind, word, self.task);
            }
            if outcome == Outcome::Quit {
                return Ok(());
            }
            let next = cycle.advance();
            if outcome == Outcome::Finished {
                let mut msg = format!("{} done, time for {}", kind.label(), next.label());
                if kind == Kind::Work && self.done_today == self.config.daily_goal {
                    msg = format!("daily goal of {} reached! {msg}", self.done_today);
                }
                notify::send(self.config.notify, &msg)?;
            }
            if self.count.is_some_and(|n| cycle.completed_work() >= n) {
                return Ok(());
            }
            if !self.config.auto_start && !self.wait_for_start(next)? {
                return Ok(());
            }
        }
    }

    fn session(&mut self, kind: Kind, timer: &mut Timer) -> io::Result<Outcome> {
        timer.start(SystemTime::now());
        self.publish(kind, timer);
        if !self.live {
            write!(
                self.out,
                "{} started ({})",
                kind.label(),
                duration::clock(timer.length())
            )?;
            match self.task.filter(|_| kind == Kind::Work) {
                Some(t) => writeln!(self.out, ": {t}")?,
                None => writeln!(self.out)?,
            }
        }
        while !timer.is_done(SystemTime::now()) {
            self.draw(kind, timer)?;
            match self.next_key() {
                Some(Key::Toggle) => timer.toggle(SystemTime::now()),
                Some(Key::Pause) => timer.pause(SystemTime::now()),
                Some(Key::Resume) => timer.start(SystemTime::now()),
                Some(Key::Extend) => timer.extend(Duration::from_secs(60)),
                Some(Key::Restart) => timer.restart(SystemTime::now()),
                Some(Key::Skip) => return Ok(Outcome::Skipped),
                Some(Key::Quit) => return Ok(Outcome::Quit),
                None => continue,
            }
            self.publish(kind, timer);
        }
        Ok(Outcome::Finished)
    }

    /// Waits up to one tick for a keypress, then checks the control file.
    /// Once stdin is closed the channel disconnects and returns instantly,
    /// so sleep in that case too rather than spinning.
    fn next_key(&self) -> Option<Key> {
        let key = match self.keys.map(|rx| rx.recv_timeout(TICK)) {
            Some(Ok(k)) => Some(k),
            Some(Err(RecvTimeoutError::Timeout)) => None,
            Some(Err(RecvTimeoutError::Disconnected)) | None => {
                thread::sleep(TICK);
                None
            }
        };
        key.or_else(|| remote::take(self.state_dir)?.parse().ok())
    }

    /// Best effort: a read-only state dir shouldn't stop the timer.
    fn publish(&self, kind: Kind, timer: &Timer) {
        let state = State {
            pid: std::process::id(),
            kind,
            length: timer.length(),
            elapsed: timer.elapsed(SystemTime::now()),
            paused: !timer.is_running(),
            at: remote::now_ms(),
        };
        let _ = remote::save(self.state_dir, &state);
    }

    /// Blocks until the user presses space (start) or q (quit). Without a
    /// terminal to read keys from there is nobody to wait for, so it
    /// starts straight away.
    fn wait_for_start(&mut self, next: Kind) -> io::Result<bool> {
        if self.keys.is_none() {
            return Ok(true);
        }
        self.publish(next, &Timer::new(next.length(self.config)));
        write!(self.out, "press space to start {}", next.label())?;
        self.out.flush()?;
        loop {
            match self.next_key() {
                Some(Key::Toggle | Key::Resume | Key::Skip) => break,
                Some(Key::Quit) => {
                    writeln!(self.out)?;
                    return Ok(false);
                }
                _ => {}
            }
        }
        write!(self.out, "{}", bar::CLEAR_LINE)?;
        Ok(true)
    }

    fn draw(&mut self, kind: Kind, timer: &Timer) -> io::Result<()> {
        if !self.live {
            return Ok(());
        }
        let now = SystemTime::now();
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
        if c.title {
            let left = duration::clock(duration::ceil_secs(timer.remaining(now)));
            write!(self.out, "{}", bar::title(&format!("{} {left}", kind.label())))?;
        }
        write!(self.out, "{}{}", bar::CLEAR_LINE, line)?;
        if let Some(t) = self.task.filter(|_| kind == Kind::Work) {
            write!(self.out, " · {t}")?;
        }
        self.out.flush()
    }
}
