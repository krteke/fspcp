use std::path::PathBuf;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum Commands {
    List,
    Copy {
        #[arg(short, long)]
        outputs: Vec<PathBuf>,
    },
}
