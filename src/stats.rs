use crate::duration::human;
use crate::log::Entry;
use crate::session::Kind;
use chrono::{Datelike, NaiveDate};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::time::Duration;

#[derive(Debug, Default, PartialEq)]
pub struct Totals {
    /// Work sessions that ran their full length.
    pub sessions: u32,
    /// All time spent in work sessions, including skipped ones.
    pub focused: Duration,
}

impl Totals {
    fn add(&mut self, e: &Entry) {
        if e.completed() {
            self.sessions += 1;
        }
        self.focused += e.actual;
    }
}

pub fn totals(entries: &[Entry]) -> Totals {
    let mut t = Totals::default();
    for e in entries.iter().filter(|e| e.kind == Kind::Work) {
        t.add(e);
    }
    t
}

/// Work totals grouped by the calendar day each session started on.
pub fn per_day(entries: &[Entry]) -> BTreeMap<NaiveDate, Totals> {
    let mut days: BTreeMap<NaiveDate, Totals> = BTreeMap::new();
    for e in entries.iter().filter(|e| e.kind == Kind::Work) {
        days.entry(e.start.date_naive()).or_default().add(e);
    }
    days
}

#[derive(Debug, Default, PartialEq)]
pub struct Streaks {
    pub current: u32,
    pub longest: u32,
}

/// Consecutive days with at least one work session. The current streak
/// survives until the end of the day after the last session, so not having
/// started yet today doesn't reset it.
pub fn streaks(days: &BTreeMap<NaiveDate, Totals>, today: NaiveDate) -> Streaks {
    let mut s = Streaks::default();
    let mut run = 0;
    let mut prev: Option<NaiveDate> = None;
    for (&d, _) in days.iter().filter(|(_, t)| t.sessions > 0) {
        run = match prev {
            Some(p) if p.succ_opt() == Some(d) => run + 1,
            _ => 1,
        };
        s.longest = s.longest.max(run);
        prev = Some(d);
    }
    if let Some(last) = prev
        && (last == today || last.succ_opt() == Some(today))
    {
        s.current = run;
    }
    s
}

pub fn best_day(days: &BTreeMap<NaiveDate, Totals>) -> Option<(NaiveDate, &Totals)> {
    days.iter()
        .max_by_key(|(d, t)| (t.focused, std::cmp::Reverse(**d)))
        .map(|(d, t)| (*d, t))
}

/// Mean focus time over the days that had any work at all.
pub fn daily_average(days: &BTreeMap<NaiveDate, Totals>) -> Duration {
    let active = days.values().filter(|t| !t.focused.is_zero()).count() as u32;
    if active == 0 {
        return Duration::ZERO;
    }
    days.values().map(|t| t.focused).sum::<Duration>() / active
}

/// Work totals per task label, most focused first. Unlabeled sessions
/// are grouped under `None`.
pub fn per_task(entries: &[Entry]) -> Vec<(Option<&str>, Totals)> {
    let mut map: BTreeMap<Option<&str>, Totals> = BTreeMap::new();
    for e in entries.iter().filter(|e| e.kind == Kind::Work) {
        map.entry(e.task.as_deref()).or_default().add(e);
    }
    let mut list: Vec<_> = map.into_iter().collect();
    list.sort_by_key(|(_, t)| std::cmp::Reverse(t.focused));
    list
}

/// Average focus per weekday, Monday first, over the days that had work.
pub fn by_weekday(days: &BTreeMap<NaiveDate, Totals>) -> [Duration; 7] {
    let mut sum = [Duration::ZERO; 7];
    let mut n = [0u32; 7];
    for (d, t) in days.iter().filter(|(_, t)| !t.focused.is_zero()) {
        let i = d.weekday().num_days_from_monday() as usize;
        sum[i] += t.focused;
        n[i] += 1;
    }
    std::array::from_fn(|i| {
        if n[i] == 0 {
            Duration::ZERO
        } else {
            sum[i] / n[i]
        }
    })
}

/// Work totals per calendar month, oldest first, as (year, month).
pub fn per_month(days: &BTreeMap<NaiveDate, Totals>) -> BTreeMap<(i32, u32), Totals> {
    let mut months: BTreeMap<(i32, u32), Totals> = BTreeMap::new();
    for (d, t) in days {
        let m = months.entry((d.year(), d.month())).or_default();
        m.sessions += t.sessions;
        m.focused += t.focused;
    }
    months
}

/// Totals since the most recent Monday, today included.
pub fn this_week(days: &BTreeMap<NaiveDate, Totals>, today: NaiveDate) -> Totals {
    let monday = today.week(chrono::Weekday::Mon).first_day();
    let mut t = Totals::default();
    for (_, d) in days.range(monday..=today) {
        t.sessions += d.sessions;
        t.focused += d.focused;
    }
    t
}

fn plural(n: u32, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

fn ratio(d: Duration, max: Duration) -> f64 {
    if max.is_zero() {
        0.0
    } else {
        d.as_secs_f64() / max.as_secs_f64()
    }
}

/// A GitHub style calendar of the last `weeks` weeks: one row per weekday,
/// one column per week, darker cells for more focus.
pub fn heatmap(entries: &[Entry], today: NaiveDate, weeks: u32) -> String {
    const SHADES: [char; 5] = ['·', '░', '▒', '▓', '█'];
    let days = per_day(entries);
    let this_monday = today.week(chrono::Weekday::Mon).first_day();
    let start = this_monday - chrono::Days::new(7 * u64::from(weeks.saturating_sub(1)));
    let max = days
        .range(start..=today)
        .map(|(_, t)| t.focused)
        .max()
        .unwrap_or_default();
    let names = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
    let mut out = String::new();
    for (row, name) in names.iter().enumerate() {
        out.push_str(name);
        out.push(' ');
        for week in 0..weeks {
            let d = start + chrono::Days::new(u64::from(week) * 7 + row as u64);
            if d > today {
                out.push(' ');
                continue;
            }
            let f = days.get(&d).map_or(Duration::ZERO, |t| t.focused);
            let level = if f.is_zero() {
                0
            } else {
                1 + ((ratio(f, max) * 4.0).ceil() as usize - 1).min(3)
            };
            out.push(SHADES[level]);
        }
        out.push('\n');
    }
    out
}

pub fn month_report(entries: &[Entry]) -> String {
    let months = per_month(&per_day(entries));
    let max = months.values().map(|t| t.focused).max().unwrap_or_default();
    let mut out = String::new();
    for ((y, m), t) in &months {
        let b = crate::bar::bar(ratio(t.focused, max), 10);
        let _ = writeln!(
            out,
            "{y}-{m:02}  {b}  {:>4}  {}",
            t.sessions,
            human(t.focused)
        );
    }
    out
}

pub fn weekday_report(entries: &[Entry]) -> String {
    let avg = by_weekday(&per_day(entries));
    let max = avg.iter().max().copied().unwrap_or_default();
    let names = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
    let mut out = String::new();
    for (name, d) in names.iter().zip(avg) {
        let b = crate::bar::bar(ratio(d, max), 10);
        let _ = writeln!(out, "{name}  {b}  {}", human(d));
    }
    out
}

pub fn report(entries: &[Entry], today: NaiveDate, last: u32, goal: u32) -> String {
    let all = totals(entries);
    let days = per_day(entries);
    let st = streaks(&days, today);
    let none = Totals::default();
    let t = days.get(&today).unwrap_or(&none);
    let mut out = String::new();
    let _ = write!(
        out,
        "today     {:>13}  {}",
        plural(t.sessions, "session"),
        human(t.focused)
    );
    if goal > 0 {
        let mark = if t.sessions >= goal { " ✓" } else { "" };
        let _ = write!(out, "  goal {}/{goal}{mark}", t.sessions);
    }
    out.push('\n');
    let w = this_week(&days, today);
    let _ = writeln!(
        out,
        "this week {:>13}  {}",
        plural(w.sessions, "session"),
        human(w.focused)
    );
    let _ = writeln!(
        out,
        "all time  {:>13}  {}",
        plural(all.sessions, "session"),
        human(all.focused)
    );
    let _ = writeln!(
        out,
        "streak    {:>13}  longest {}",
        plural(st.current, "day"),
        st.longest
    );
    if let Some((d, t)) = best_day(&days) {
        let _ = writeln!(out, "best day  {:>13}  {}", d.to_string(), human(t.focused));
        let _ = writeln!(out, "daily avg {:>13}", human(daily_average(&days)));
    }
    let tasks = per_task(entries);
    if tasks.iter().any(|(t, _)| t.is_some()) {
        let _ = writeln!(out, "\nby task");
        for (name, t) in &tasks {
            let _ = writeln!(
                out,
                "{:<20}  {:>3}  {}",
                name.unwrap_or("(none)"),
                t.sessions,
                human(t.focused)
            );
        }
    }
    if last > 0 {
        let _ = writeln!(out, "\nlast {last} days");
        let recent: Vec<(NaiveDate, &Totals)> = recent(today, last)
            .map(|d| (d, days.get(&d).unwrap_or(&none)))
            .collect();
        let max = recent
            .iter()
            .map(|(_, t)| t.focused)
            .max()
            .unwrap_or_default();
        for (d, t) in recent {
            let _ = writeln!(
                out,
                "{d}  {}  {:>3}  {}",
                crate::bar::bar(ratio(t.focused, max), 10),
                t.sessions,
                human(t.focused)
            );
        }
    }
    out
}

fn recent(today: NaiveDate, last: u32) -> impl Iterator<Item = NaiveDate> {
    (0..last)
        .rev()
        .filter_map(move |i| today.checked_sub_days(chrono::Days::new(i.into())))
}

fn totals_json(t: &Totals) -> String {
    format!(
        "{{\"sessions\":{},\"focused_secs\":{}}}",
        t.sessions,
        t.focused.as_secs()
    )
}

/// Same numbers as [`report`], as a single line of JSON for scripts.
pub fn json(entries: &[Entry], today: NaiveDate, last: u32) -> String {
    let days = per_day(entries);
    let st = streaks(&days, today);
    let none = Totals::default();
    let list: Vec<String> = recent(today, last)
        .map(|d| {
            let t = days.get(&d).unwrap_or(&none);
            format!("{{\"date\":\"{d}\",\"totals\":{}}}", totals_json(t))
        })
        .collect();
    format!(
        "{{\"today\":{},\"all_time\":{},\"streak\":{{\"current\":{},\"longest\":{}}},\"days\":[{}]}}",
        totals_json(days.get(&today).unwrap_or(&none)),
        totals_json(&totals(entries)),
        st.current,
        st.longest,
        list.join(",")
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chrono::DateTime;

    pub fn entry(start: &str, kind: Kind, planned: u64, actual: u64) -> Entry {
        Entry {
            start: DateTime::parse_from_rfc3339(start).unwrap(),
            kind,
            planned: Duration::from_secs(planned),
            actual: Duration::from_secs(actual),
            task: None,
        }
    }

    #[test]
    fn totals_count_only_work() {
        let log = [
            entry("2026-11-17T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-17T09:25:00Z", Kind::ShortBreak, 300, 300),
            entry("2026-11-17T09:30:00Z", Kind::Work, 1500, 1500),
        ];
        let t = totals(&log);
        assert_eq!(t.sessions, 2);
        assert_eq!(t.focused, Duration::from_secs(3000));
    }

    #[test]
    fn skipped_sessions_count_time_but_not_sessions() {
        let log = [
            entry("2026-11-17T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-17T10:00:00Z", Kind::Work, 1500, 300),
        ];
        let t = totals(&log);
        assert_eq!(t.sessions, 1);
        assert_eq!(t.focused, Duration::from_secs(1800));
    }

    fn day(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn per_day_groups_by_local_date() {
        let log = [
            entry("2026-11-16T23:30:00-08:00", Kind::Work, 1500, 1500),
            entry("2026-11-17T08:00:00-08:00", Kind::Work, 1500, 600),
            entry("2026-11-17T09:00:00-08:00", Kind::Work, 1500, 1500),
        ];
        let days = per_day(&log);
        assert_eq!(days.len(), 2);
        assert_eq!(days[&day("2026-11-16")].sessions, 1);
        assert_eq!(days[&day("2026-11-17")].sessions, 1);
        assert_eq!(days[&day("2026-11-17")].focused, Duration::from_secs(2100));
    }

    fn days(list: &[&str]) -> BTreeMap<NaiveDate, Totals> {
        let one = || Totals {
            sessions: 1,
            focused: Duration::from_secs(1500),
        };
        list.iter().map(|d| (day(d), one())).collect()
    }

    #[test]
    fn days_with_only_skipped_work_break_the_streak() {
        let mut d = days(&["2026-11-15", "2026-11-17"]);
        d.insert(day("2026-11-16"), Totals::default());
        assert_eq!(
            streaks(&d, day("2026-11-17")),
            Streaks {
                current: 1,
                longest: 1
            }
        );
    }

    #[test]
    fn streak_counts_consecutive_days() {
        let d = days(&[
            "2026-11-10",
            "2026-11-11",
            "2026-11-12",
            "2026-11-15",
            "2026-11-16",
        ]);
        let s = streaks(&d, day("2026-11-16"));
        assert_eq!(
            s,
            Streaks {
                current: 2,
                longest: 3
            }
        );
    }

    #[test]
    fn streak_survives_until_tomorrow_ends() {
        let d = days(&["2026-11-15", "2026-11-16"]);
        assert_eq!(streaks(&d, day("2026-11-17")).current, 2);
        assert_eq!(streaks(&d, day("2026-11-18")).current, 0);
    }

    #[test]
    fn no_days_no_streak() {
        assert_eq!(
            streaks(&BTreeMap::new(), day("2026-11-17")),
            Streaks::default()
        );
    }

    #[test]
    fn report_layout() {
        let log = [
            entry("2026-11-16T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-17T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-17T10:00:00Z", Kind::Work, 1500, 1500),
        ];
        let r = report(&log, day("2026-11-17"), 3, 0);
        let expected = "\
today        2 sessions  50m
this week    3 sessions  1h15m
all time     3 sessions  1h15m
streak           2 days  longest 2
best day     2026-11-17  50m
daily avg           37m

last 3 days
2026-11-15  ░░░░░░░░░░    0  0m
2026-11-16  █████░░░░░    1  25m
2026-11-17  ██████████    2  50m
";
        assert_eq!(r, expected);
    }

    #[test]
    fn best_day_prefers_most_focus_then_earliest() {
        let log = [
            entry("2026-11-15T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-16T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-16T10:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-17T09:00:00Z", Kind::Work, 3000, 3000),
        ];
        let days = per_day(&log);
        let (d, t) = best_day(&days).unwrap();
        assert_eq!(d, day("2026-11-16"));
        assert_eq!(t.sessions, 2);
        assert!(best_day(&BTreeMap::new()).is_none());
    }

    #[test]
    fn average_ignores_empty_days() {
        let log = [
            entry("2026-11-15T09:00:00Z", Kind::Work, 600, 600),
            entry("2026-11-17T09:00:00Z", Kind::Work, 1800, 1800),
        ];
        assert_eq!(daily_average(&per_day(&log)), Duration::from_secs(1200));
        assert_eq!(daily_average(&BTreeMap::new()), Duration::ZERO);
    }

    #[test]
    fn week_starts_on_monday() {
        let log = [
            entry("2026-11-15T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-16T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-18T09:00:00Z", Kind::Work, 1500, 1500),
        ];
        let w = this_week(&per_day(&log), day("2026-11-18"));
        assert_eq!(w.sessions, 2);
    }

    fn tagged(start: &str, task: &str, secs: u64) -> Entry {
        Entry {
            task: Some(task.into()),
            ..entry(start, Kind::Work, secs, secs)
        }
    }

    #[test]
    fn per_task_sorts_by_focus() {
        let log = [
            tagged("2026-11-17T09:00:00Z", "docs", 600),
            tagged("2026-11-17T10:00:00Z", "code", 1500),
            entry("2026-11-17T11:00:00Z", Kind::Work, 900, 900),
            tagged("2026-11-17T12:00:00Z", "docs", 600),
        ];
        let tasks = per_task(&log);
        let names: Vec<_> = tasks.iter().map(|(t, _)| *t).collect();
        assert_eq!(names, [Some("code"), Some("docs"), None]);
        assert_eq!(tasks[1].1.sessions, 2);
    }

    #[test]
    fn report_lists_tasks_only_when_labelled() {
        let plain = [entry("2026-11-17T09:00:00Z", Kind::Work, 1500, 1500)];
        assert!(!report(&plain, day("2026-11-17"), 0, 0).contains("by task"));
        let labelled = [tagged("2026-11-17T09:00:00Z", "code", 1500)];
        let r = report(&labelled, day("2026-11-17"), 0, 0);
        assert!(
            r.contains("\nby task\ncode                    1  25m\n"),
            "{r}"
        );
    }

    #[test]
    fn goal_progress_on_the_today_line() {
        let log = [entry("2026-11-17T09:00:00Z", Kind::Work, 1500, 1500)];
        let first = |goal| {
            report(&log, day("2026-11-17"), 0, goal)
                .lines()
                .next()
                .unwrap()
                .to_string()
        };
        assert!(first(4).ends_with("goal 1/4"), "{}", first(4));
        assert!(first(1).ends_with("goal 1/1 ✓"), "{}", first(1));
        assert!(!first(0).contains("goal"));
    }

    #[test]
    fn weekday_averages() {
        let log = [
            entry("2026-11-16T09:00:00Z", Kind::Work, 600, 600),
            entry("2026-11-23T09:00:00Z", Kind::Work, 1800, 1800),
            entry("2026-11-18T09:00:00Z", Kind::Work, 900, 900),
        ];
        let w = by_weekday(&per_day(&log));
        assert_eq!(w[0], Duration::from_secs(1200));
        assert_eq!(w[1], Duration::ZERO);
        assert_eq!(w[2], Duration::from_secs(900));
    }

    #[test]
    fn weekday_report_layout() {
        let log = [entry("2026-11-17T09:00:00Z", Kind::Work, 1500, 1500)];
        let r = weekday_report(&log);
        assert_eq!(r.lines().count(), 7);
        assert_eq!(r.lines().nth(1).unwrap(), "tue  ██████████  25m");
        assert_eq!(r.lines().next().unwrap(), "mon  ░░░░░░░░░░  0m");
    }

    #[test]
    fn heatmap_grid() {
        let log = [
            entry("2026-11-10T09:00:00Z", Kind::Work, 3000, 3000),
            entry("2026-11-17T09:00:00Z", Kind::Work, 600, 600),
        ];
        let h = heatmap(&log, day("2026-11-17"), 2);
        let expected = "\
mon ··
tue █░
wed · \nthu · \nfri · \nsat · \nsun · \n";
        assert_eq!(h, expected);
    }

    #[test]
    fn plural_words() {
        assert_eq!(plural(1, "day"), "1 day");
        assert_eq!(plural(0, "day"), "0 days");
        assert_eq!(plural(2, "session"), "2 sessions");
    }

    #[test]
    fn json_output() {
        let log = [entry("2026-11-17T09:00:00Z", Kind::Work, 1500, 1500)];
        let j = json(&log, day("2026-11-17"), 2);
        assert_eq!(
            j,
            r#"{"today":{"sessions":1,"focused_secs":1500},"all_time":{"sessions":1,"focused_secs":1500},"streak":{"current":1,"longest":1},"days":[{"date":"2026-11-16","totals":{"sessions":0,"focused_secs":0}},{"date":"2026-11-17","totals":{"sessions":1,"focused_secs":1500}}]}"#
        );
    }

    #[test]
    fn months_roll_up_days() {
        let log = [
            entry("2026-10-31T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-01T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-17T09:00:00Z", Kind::Work, 1500, 1500),
        ];
        let m = per_month(&per_day(&log));
        assert_eq!(m.len(), 2);
        assert_eq!(m[&(2026, 10)].sessions, 1);
        assert_eq!(m[&(2026, 11)].sessions, 2);
    }

    #[test]
    fn month_report_layout() {
        let log = [
            entry("2026-10-31T09:00:00Z", Kind::Work, 1500, 1500),
            entry("2026-11-01T09:00:00Z", Kind::Work, 3000, 3000),
        ];
        assert_eq!(
            month_report(&log),
            "2026-10  █████░░░░░     1  25m\n2026-11  ██████████     1  50m\n"
        );
    }
}
