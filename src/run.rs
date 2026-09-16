//! The interactive loop behind `pomo start`.

use crate::bar;
use crate::cli::StartArgs;
use crate::config::{Config, Display};
use crate::duration;
use crate::hook;
use crate::keys::{self, Key};
use crate::log::{self, Entry};
use crate::notify;
use crate::remote::{self, State};
use crate::session::{Cycle, Kind};
use crate::timer::Timer;
use std::collections::VecDeque;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, SystemTime};

const TICK: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Finished,
    /// Finished, then kept going past zero.
    Overtime,
    Skipped,
    Quit,
}

struct Runner<'a, W: Write> {
    config: &'a Config,
    log_path: &'a Path,
    state_dir: &'a Path,
    task: Option<&'a str>,
    count: Option<u32>,
    first: Kind,
    deadline: Option<chrono::NaiveDateTime>,
    keys: Option<&'a Receiver<Key>>,
    live: bool,
    done_today: u32,
    auto_width: Option<usize>,
    queued: VecDeque<Key>,
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

pub fn run(config: &Config, args: &StartArgs, log_path: &Path, state_dir: &Path) -> io::Result<()> {
    let out = io::stdout();
    let live = out.is_terminal();
    let hide = config.display == Display::Bar && live;
    remote::take(state_dir);
    let listener = keys::listen();
    let mut r = Runner {
        config,
        log_path,
        state_dir,
        task: args.task.as_deref(),
        count: args.count,
        first: args.first,
        deadline: args
            .until
            .map(|u| crate::cli::deadline(chrono::Local::now().naive_local(), u)),
        keys: listener.as_ref().map(|(_, rx)| rx),
        live,
        done_today: done_today(log_path),
        auto_width: None,
        queued: VecDeque::new(),
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
        let mut cycle = Cycle::starting_with(self.config.long_break_every, self.first);
        loop {
            if self
                .deadline
                .is_some_and(|d| chrono::Local::now().naive_local() >= d)
            {
                writeln!(self.out, "reached --until, stopping")?;
                return Ok(());
            }
            let kind = cycle.current();
            let mut timer = Timer::new(kind.length(self.config));
            let started = chrono::Local::now().fixed_offset();
            let outcome = self.session(kind, &mut timer)?;
            let actual = match outcome {
                Outcome::Finished => timer.length(),
                Outcome::Overtime => timer.elapsed(SystemTime::now()),
                _ => timer.elapsed(SystemTime::now()).min(timer.length()),
            };
            let word = match outcome {
                Outcome::Finished | Outcome::Overtime => "done",
                Outcome::Skipped => "skipped",
                Outcome::Quit => "stopped",
            };
            if self.live {
                write!(self.out, "{}", bar::CLEAR_LINE)?;
            }
            write!(self.out, "{} {word}", kind.label())?;
            let goal = self.config.daily_goal;
            if kind == Kind::Work && matches!(outcome, Outcome::Finished | Outcome::Overtime) {
                self.done_today += 1;
                if goal > 0 {
                    write!(self.out, " ({}/{goal} today)", self.done_today)?;
                }
            }
            writeln!(self.out)?;
            let entry = Entry {
                start: started,
                kind,
                planned: timer.length(),
                actual,
                task: self.task.map(str::to_string),
            };
            if let Err(e) = log::append(self.log_path, &entry) {
                eprintln!("pomo: could not write {}: {e}", self.log_path.display());
                eprintln!("pomo: unsaved session: {}", entry.to_line());
            }
            if let Some(cmd) = &self.config.hook {
                hook::run(cmd, kind, word, self.task, actual);
            }
            if outcome == Outcome::Quit {
                return Ok(());
            }
            let next = cycle.advance();
            if outcome == Outcome::Finished {
                let mut msg = format!("{} done, time for {}", kind.label(), next.label());
                if let Some(t) = self.task.filter(|_| kind == Kind::Work) {
                    msg = format!("{t}: {msg}");
                }
                if kind == Kind::Work && self.done_today == self.config.daily_goal {
                    msg = format!("daily goal of {} reached! {msg}", self.done_today);
                }
                self.notify(&msg);
            }
            if self.count.is_some_and(|n| cycle.completed_work() >= n) {
                return Ok(());
            }
            if !self.config.starts_automatically(next) && !self.wait_for_start(next)? {
                return Ok(());
            }
        }
    }

    fn session(&mut self, kind: Kind, timer: &mut Timer) -> io::Result<Outcome> {
        timer.start(SystemTime::now());
        if self.config.bar_width == 0 {
            self.auto_width = bar::term_cols().map(|c| bar::fit_width(c, kind.label()));
        }
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
        let mut reminded = false;
        while !timer.is_done(SystemTime::now()) {
            self.draw(kind, timer)?;
            self.remind_if_paused(kind, timer, &mut reminded);
            match self.next_key() {
                Some(Key::Toggle) => timer.toggle(SystemTime::now()),
                Some(Key::Pause) => timer.pause(SystemTime::now()),
                Some(Key::Resume) => timer.start(SystemTime::now()),
                Some(Key::Extend(by)) => timer.extend(by),
                Some(Key::Restart) => timer.restart(SystemTime::now()),
                Some(Key::Skip) => return Ok(Outcome::Skipped),
                Some(Key::Quit) => return Ok(Outcome::Quit),
                None => continue,
            }
            self.publish(kind, timer);
        }
        if kind == Kind::Work && self.config.overtime {
            return self.overtime(kind, timer);
        }
        Ok(Outcome::Finished)
    }

    /// Counts up past zero until the user moves on. Pausing doesn't make
    /// sense here, so space and s both end the session.
    fn overtime(&mut self, kind: Kind, timer: &Timer) -> io::Result<Outcome> {
        self.notify(&format!("{} done, overtime running", kind.label()));
        loop {
            if self.live {
                let over = timer
                    .elapsed(SystemTime::now())
                    .saturating_sub(timer.length());
                let line = format!("{} +{} overtime", kind.label(), duration::clock(over));
                write!(self.out, "{}{line}", bar::CLEAR_LINE)?;
                self.out.flush()?;
            }
            match self.next_key() {
                Some(Key::Skip | Key::Toggle | Key::Resume) => return Ok(Outcome::Overtime),
                Some(Key::Quit) => return Ok(Outcome::Quit),
                _ => {}
            }
        }
    }

    /// Waits up to one tick for a keypress, then checks the control file.
    /// Once stdin is closed the channel disconnects and returns instantly,
    /// so sleep in that case too rather than spinning.
    fn next_key(&mut self) -> Option<Key> {
        if let Some(k) = self.queued.pop_front() {
            return Some(k);
        }
        let key = match self.keys.map(|rx| rx.recv_timeout(TICK)) {
            Some(Ok(k)) => Some(k),
            Some(Err(RecvTimeoutError::Timeout)) => None,
            Some(Err(RecvTimeoutError::Disconnected)) | None => {
                thread::sleep(TICK);
                None
            }
        };
        if key.is_some() {
            return key;
        }
        let cmds = remote::take(self.state_dir);
        self.queued
            .extend(cmds.iter().filter_map(|c| c.parse::<Key>().ok()));
        self.queued.pop_front()
    }

    /// Sends one nudge per pause once it has lasted `pause_reminder`.
    fn remind_if_paused(&self, kind: Kind, timer: &Timer, reminded: &mut bool) {
        let limit = self.config.pause_reminder;
        let now = SystemTime::now();
        match timer.paused_for(now) {
            None => *reminded = false,
            Some(p) if !limit.is_zero() && p >= limit && !*reminded => {
                let left = duration::clock(duration::ceil_secs(timer.remaining(now)));
                self.notify(&format!("{} still paused, {left} left", kind.label()));
                *reminded = true;
            }
            Some(_) => {}
        }
    }

    fn notify(&self, msg: &str) {
        let c = self.config;
        if let Err(e) = notify::send(c.notify, msg, c.sound.as_deref()) {
            eprintln!("pomo: notification failed: {e}");
        }
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
            Display::Bar => {
                let width = match c.bar_width {
                    0 => self.auto_width.unwrap_or(24),
                    w => w,
                };
                let b = bar::render(c.bar_style, timer.progress(now), width);
                bar::frame(kind, b, timer.remaining(now), !timer.is_running(), c.color)
            }
        };
        if c.title {
            let left = duration::clock(duration::ceil_secs(timer.remaining(now)));
            write!(
                self.out,
                "{}",
                bar::title(&format!("{} {left}", kind.label()))
            )?;
        }
        write!(self.out, "{}{}", bar::CLEAR_LINE, line)?;
        if let Some(t) = self.task.filter(|_| kind == Kind::Work) {
            write!(self.out, " · {t}")?;
        }
        self.out.flush()
    }
}
