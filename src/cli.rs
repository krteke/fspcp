use anyhow::Result;
use clap::Parser;
use crossfire::{
    MTx,
    mpsc::{self, Array},
};
use ignore::WalkBuilder;
use parse_size::parse_size;
use std::{ffi::OsStr, mem, path::PathBuf, sync::Arc, thread};

use crate::cli::{args::Args, commands::Commands};

mod args;
mod commands;

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    args: Args,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let mut args = self.args;
        tracing::info!("Args: \n{}", args);

        if args.dirs.is_empty() {
            return Ok(());
        }

        args.process_paths();

        let excludes = Arc::new(args.excludes);

        let mut walker = WalkBuilder::from_iter(args.dirs);

        walker
            .standard_filters(false)
            .hidden(args.hide)
            .filter_entry({
                let excludes = excludes.clone();

                move |entry| !excludes.iter().any(|e| entry.path().starts_with(e))
            });

        let (tx, rx) = mpsc::bounded_blocking::<Vec<PathBuf>>(1024);
        let exts = Arc::new(args.extensions);

        let files = thread::spawn(move || {
            let mut files = Vec::new();

            while let Ok(value) = rx.recv() {
                files.extend(value);
            }

            files
        });

        let size = args.size.map_or(0, |size| {
            parse_size(&size).unwrap_or_else(|e| {
                tracing::warn!("invalid size {}: {}", size, e);
                0
            })
        });

        walker.build_parallel().run(|| {
            let exts = exts.clone();
            let mut thread_files = ThreadFiles {
                files: Vec::with_capacity(1024),
                sender: tx.clone(),
            };

            Box::new(move |result| {
                match result {
                    Ok(entry) => {
                        let Some(file_type) = entry.file_type() else {
                            tracing::info!(
                                "Skip {} because of unknown file type ",
                                entry.file_name().display()
                            );
                            return ignore::WalkState::Continue;
                        };

                        let metadata = entry.metadata();
                        let path = entry.into_path();

                        if file_type.is_file()
                            && let Some(ext) = path.extension()
                            && (exts.is_empty() || exts.iter().any(|e| OsStr::new(e) == ext))
                            && let Ok(metadata) = metadata
                            && metadata.len() >= size
                        {
                            thread_files.files.push(path);
                        }
                    }
                    Err(e) => tracing::error!("{e}"),
                }

                ignore::WalkState::Continue
            })
        });

        drop(tx);

        let files = files.join().unwrap_or_else(|e| {
            tracing::warn!("{e:?}");
            Vec::new()
        });

        self.command.run(&files)
    }

    pub fn verbose(&self) -> u8 {
        self.args.verbose
    }
}

struct ThreadFiles {
    files: Vec<PathBuf>,
    sender: MTx<Array<Vec<PathBuf>>>,
}

impl Drop for ThreadFiles {
    fn drop(&mut self) {
        if self.files.is_empty() {
            return;
        }

        let files = mem::take(&mut self.files);
        if let Err(e) = self.sender.send(files) {
            tracing::warn!("{e}")
        }
    }
}
