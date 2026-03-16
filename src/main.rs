mod cli;
mod config;
mod duration;
mod paths;

use clap::Parser;

fn main() {
    let _cli = cli::Cli::parse();
}
