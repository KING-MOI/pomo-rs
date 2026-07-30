# Contributing

Thanks for taking a look! pomo is small on purpose, so the bar for new
features is "would most people who run a pomodoro timer want this?".

## Development

```sh
cargo test                 # unit + integration tests
cargo clippy --all-targets -- -D warnings
cargo fmt
```

Integration tests in `tests/cli.rs` run the real binary against a throwaway
home directory, so they never touch your own config or log.

## Guidelines

- Keep the dependency list short. If a few lines of std do the job, prefer them.
- Timer logic takes `now` as an argument; keep it that way so it stays testable
  without sleeping.
- The log format is append-only and tab separated. New fields go at the end and
  must stay optional so old logs keep parsing.
- One logical change per commit, with a short lowercase message
  (`feat: ...`, `fix: ...`, `docs: ...`).
