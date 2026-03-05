use clap::Parser;

#[derive(Parser)]
#[command(name = "pomo", version, about = "A minimal pomodoro timer for the terminal")]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
