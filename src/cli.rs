use anyhow::{Result, anyhow};
use clap::Parser;
use crossfire::mpmc;
use ignore::WalkBuilder;
use parse_size::parse_size;
use std::{
    ffi::OsStr,
    path::PathBuf,
    sync::Arc,
    thread::{self, JoinHandle},
};

use crate::cli::{args::Args, batch::BatchSender, commands::Commands};

mod args;
mod batch;
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
        let cpus = num_cpus::get();

        let (tx, rx) = mpmc::bounded_blocking::<Vec<PathBuf>>(cpus * 16);
        let (ptx, prx) = mpmc::bounded_blocking::<Vec<PathBuf>>(cpus);
        let exts = Arc::new(args.extensions);

        let recv = match self.command {
            Commands::List => (0..1)
                .map(|_| {
                    let prx = prx.clone();
                    thread::spawn(move || {
                        while let Ok(p) = prx.recv() {
                            p.iter().for_each(|p| {
                                println!("{}", p.display());
                            });
                        }
                    })
                })
                .collect::<Vec<_>>(),
            Commands::Copy { outputs } => {
                if outputs.is_empty() {
                    return Ok(());
                }
                let outputs = Arc::new(outputs);

                (0..cpus)
                    .map(|_| {
                        let prx = prx.clone();
                        thread::spawn(move || todo!())
                    })
                    .collect::<Vec<_>>()
            }
        };

        let handles = (0..cpus)
            .map(|_| {
                let rx = rx.clone();
                let ptx = ptx.clone();

                thread::spawn(move || {
                    while let Ok(mut path) = rx.recv() {
                        path.retain(|p| {
                            imagesize::size(p).is_ok_and(|s| {
                                let min = s.width >= args.min_width && s.height >= args.min_height;
                                let maxw =
                                    args.max_width.is_none_or(|w| s.width <= w.get() as usize);
                                let maxh =
                                    args.max_height.is_none_or(|h| s.height <= h.get() as usize);

                                min && maxw && maxh
                            })
                        });

                        if let Err(e) = ptx.send(path) {
                            tracing::warn!("{e}")
                        }
                    }
                })
            })
            .collect::<Vec<_>>();

        let size = args.size.map_or(0, |size| {
            parse_size(&size).unwrap_or_else(|e| {
                tracing::warn!("invalid size {}: {}", size, e);
                0
            })
        });

        walker.build_parallel().run(|| {
            let exts = exts.clone();
            let mut batch_sender = BatchSender::new(tx.clone());

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
                            batch_sender.push(path);
                        }
                    }
                    Err(e) => tracing::error!("{e}"),
                }

                ignore::WalkState::Continue
            })
        });

        drop(tx);
        drop(ptx);

        let join_handle = |handles: Vec<JoinHandle<()>>| -> Result<()> {
            for handle in handles {
                handle
                    .join()
                    .map_err(|e| anyhow!("join handle error: {:?}", e))?;
            }
            Ok(())
        };

        join_handle(handles)?;
        join_handle(recv)
    }

    pub fn verbose(&self) -> u8 {
        self.args.verbose
    }
}
