mod cli;
mod commands;
mod modules;
mod output;
mod runner;
mod runtime;

use anyhow::Result;
use clap::Parser;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    runtime::init_tracing();
    let args = cli::Cli::parse();
    let status = commands::dispatch(args)?;
    std::process::exit(status.code());
}
