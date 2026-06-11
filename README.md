# pomo-rs

[![ci](https://github.com/OWNER/pomo-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/OWNER/pomo-rs/actions/workflows/ci.yml)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![crates.io](https://img.shields.io/badge/crates.io-coming%20soon-lightgrey.svg)](#)

A small pomodoro timer for the terminal, written in Rust.

## Install

```sh
cargo install --path .
```

This installs a binary called `pomo`.

## Usage

```sh
pomo                 # same as `pomo start`
pomo start -w 50m -s 10m
pomo start --bar     # progress bar instead of a plain countdown
pomo start -n 4      # stop after four work sessions
pomo stats           # focus time, streaks, last 7 days
```

While the timer runs:

| key       | action                         |
|-----------|--------------------------------|
| `space`/`p` | pause / resume               |
| `s`       | skip to the next session       |
| `+`       | add a minute                   |
| `r`       | restart the current session    |
| `q`       | stop                           |

## Configuration

pomo reads `~/.config/pomo/config.toml` (or `$XDG_CONFIG_HOME/pomo/config.toml`,
or whatever `POMO_CONFIG` points at). Every key is optional.

```sh
pomo config init   # write a commented starter file
pomo config show   # print the effective config
pomo config path   # print where it's read from
```

```toml
work = "25m"
short_break = "5m"
long_break = "15m"
long_break_every = 4
auto_start = true
display = "bar"        # or "plain"
bar_width = 24
color = true
notify = "auto"        # auto, osc9, terminal-notifier, bell, none
log_file = "~/notes/pomo.log"
```

Durations take plain minutes (`25`) or strings such as `"25m"`, `"1h30m"` and
`"90s"`. Command line flags win over the file.

## Stats

Every session is appended to `~/.local/share/pomo/sessions.log` (override with
`log_file` or `POMO_LOG`). `pomo stats` reads it back:

```
today         3 sessions  1h15m
this week    11 sessions  4h35m
all time     42 sessions  17h30m
streak           4 days  longest 9
best day     2026-11-12  3h20m
daily avg           1h45m

last 7 days
2026-11-11  ████░░░░░░    2  50m
...
```

Only full-length work sessions count as sessions; skipped ones still add
their minutes to focus time. A streak is consecutive days with at least one
full session, and it isn't broken until a whole day passes without one.

Flags: `--days N`, `--task NAME`, `--since YYYY-MM-DD`, `--json`.

## License

MIT
