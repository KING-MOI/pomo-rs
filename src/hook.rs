use crate::session::Kind;
use std::process::{Command, Stdio};

/// Runs the user's `hook` command through `sh -c` with details about the
/// session that just ended in the environment. Fire and forget: a slow or
/// broken hook must never stall the timer.
pub fn run(cmd: &str, kind: Kind, outcome: &str, task: Option<&str>) {
    let mut c = Command::new("sh");
    c.args(["-c", cmd])
        .env("POMO_KIND", kind.key())
        .env("POMO_OUTCOME", outcome)
        .env("POMO_TASK", task.unwrap_or(""))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Ok(mut child) = c.spawn() {
        std::thread::spawn(move || child.wait());
    }
}
