use std::io::IsTerminal;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt};

use crate::cli::Cli;

mod cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose())?;

    cli.run()?;

    Ok(())
}

fn init_tracing(verbose: u8) -> Result<()> {
    let filter = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(filter))
        .context("init env filter failed ")?;

    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_target(verbose >= 2)
        .with_file(verbose >= 3)
        .with_line_number(verbose >= 3)
        .try_init()
        .map_err(|e| anyhow!("init tracing_subscriber failed: {} ", e))?;

    Ok(())
}
