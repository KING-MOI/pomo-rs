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

## License

MIT
