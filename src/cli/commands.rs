use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum Commands {
    List {
        /// print total counts
        #[arg(short, long, default_value_t = false)]
        count: bool,
    },
}

impl Commands {
    pub fn run(&self, files: &[PathBuf]) -> Result<()> {
        match self {
            Commands::List { count } => {
                files.iter().for_each(|f| println!("{}", f.display()));
                if *count {
                    println!("{}", files.len());
                }
            }
        }

        Ok(())
    }
}
