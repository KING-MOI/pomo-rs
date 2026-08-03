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

Shell completions:

```sh
pomo completions zsh > ~/.zfunc/_pomo
pomo completions bash > ~/.local/share/bash-completion/completions/pomo
pomo completions fish > ~/.config/fish/completions/pomo.fish
```

## Usage

```sh
pomo                 # same as `pomo start`
pomo -w 50m -s 10m   # start flags work without `start`
pomo start -t "write report"   # label sessions for stats
pomo start -f short  # begin with a break
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
bar_width = 24         # 0 fits the bar to the terminal
color = true
title = false          # mirror the countdown in the window title
daily_goal = 8         # sessions per day, shown in stats; 0 = off
sound = "Glass"        # terminal-notifier only
hook = "say done"      # see below
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

Two other views:

```
$ pomo stats --heatmap 6        $ pomo stats --weekdays
mon ·░▒·▓█                      mon  ██████████  2h05m
tue ░▒▓█▒·                      tue  ████████░░  1h40m
...                             ...
```

## Notifications

When a session ends pomo notifies you with `notify`:

- `auto` (default): `terminal-notifier` if it's installed, otherwise OSC 9
- `osc9`: an OSC 9 escape sequence, shown as a desktop notification by
  iTerm2, WezTerm, kitty, foot and friends. Forwarded through tmux.
- `terminal-notifier`: macOS notification center
- `bell`: the terminal bell
- `none`

## Hooks

Set `hook` to run a shell command whenever a session ends:

```toml
hook = "echo \"$POMO_KIND $POMO_OUTCOME $POMO_SECONDS\" >> ~/pomo-hooks.log"
```

| variable       | value                                   |
|----------------|-----------------------------------------|
| `POMO_KIND`    | `work`, `short_break` or `long_break`   |
| `POMO_OUTCOME` | `done`, `skipped` or `stopped`          |
| `POMO_SECONDS` | seconds actually spent                  |
| `POMO_TASK`    | the `--task` label, or empty            |

The hook runs in the background; its output is discarded.

## Controlling a running timer

From another terminal (or a hotkey, or a status bar):

```sh
pomo status                       # work 12:34
pomo status -f '{kind} {remaining}'
pomo pause | resume | toggle | skip | stop
```

`--format` understands `{kind}`, `{remaining}`, `{state}` and `{percent}` and
prints nothing when no timer is running, which keeps status bars clean.

## License

MIT
