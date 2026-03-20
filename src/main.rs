mod cli;
mod config;
mod duration;
mod paths;
mod session;
mod timer;

use clap::Parser;

fn main() {
    let _cli = cli::Cli::parse();
}
