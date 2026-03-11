mod cli;
mod config;
mod duration;

use clap::Parser;

fn main() {
    let _cli = cli::Cli::parse();
}
