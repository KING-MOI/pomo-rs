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

## License

MIT
