# Status bar recipes

`pomo status --format` prints nothing when no timer is running, so these
snippets stay blank between sessions.

## tmux

```tmux
set -g status-interval 1
set -g status-right '#(pomo status -f "{kind} {remaining}")'
```

## sketchybar

```sh
sketchybar --add item pomo right \
           --set pomo update_freq=1 script='sketchybar --set pomo label="$(pomo status -f "{remaining}")"'
```

## waybar

```json
"custom/pomo": {
  "exec": "pomo status -f '{kind} {remaining}'",
  "interval": 1,
  "on-click": "pomo toggle"
}
```

## polybar

```ini
[module/pomo]
type = custom/script
exec = pomo status -f "{kind} {remaining}"
interval = 1
click-left = pomo toggle
```

## Machine readable

`pomo status --json` prints a single object, or `null` when idle:

```json
{"kind":"work","remaining_secs":754,"length_secs":1500,"paused":false,"pid":4242}
```
