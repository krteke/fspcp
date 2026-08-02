use std::{fmt::Display, num::NonZeroU32, path::PathBuf};

use clap::ArgAction;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[arg(short, long, global = true)]
    pub dirs: Vec<PathBuf>,

    #[arg(short, long, value_delimiter = ',', global = true)]
    pub extensions: Vec<String>,

    /// file >= size
    #[arg(short, long, global = true)]
    pub size: Option<String>,

    #[arg(long, global = true, default_value_t = 0)]
    pub min_width: u32,

    #[arg(long, global = true, default_value_t = 0)]
    pub min_height: u32,

    #[arg(long, global = true)]
    pub max_width: Option<NonZeroU32>,

    #[arg(long, global = true)]
    pub max_height: Option<NonZeroU32>,

    #[arg(short = 'E', long, global = true)]
    pub excludes: Vec<PathBuf>,

    /// ignore hidden files
    #[arg(short = 'H', long, global = true, default_value_t = false)]
    pub hide: bool,

    #[arg(
        short,
        long,
        global = true,
        action = ArgAction::Count
    )]
    pub verbose: u8,
}

impl Args {
    pub fn process_paths(&mut self) {
        canonicalize(&mut self.dirs);
        canonicalize(&mut self.excludes);

        let mut results = Vec::with_capacity(self.dirs.len());

        self.dirs.iter().for_each(|dir| {
            if !results.iter().any(|d| dir.starts_with(d)) {
                results.push(dir.clone());
            }
        });

        self.dirs = results;
    }
}

impl Display for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.dirs.is_empty() {
            f.write_str("Directories: ")?;
        }
        for dir in self.dirs.iter() {
            writeln!(f, "{}", dir.display())?;
        }

        if !self.extensions.is_empty() {
            f.write_str("Extensions: ")?;
            writeln!(f, "{}", self.extensions.join(", "))?;
        }

        if let Some(size) = self.size.as_ref() {
            writeln!(f, "Size >= {}", size)?;
        }

        let map_or = |s: Option<NonZeroU32>| s.map_or(String::from("-"), |s| s.to_string());

        let maxw = map_or(self.max_width);
        let maxh = map_or(self.max_height);

        writeln!(
            f,
            "Pixel range: {}x{} px - {}x{} px",
            self.min_width, self.min_height, maxw, maxh
        )?;

        if !self.excludes.is_empty() {
            f.write_str("Excludes: ")?;
        }
        for dir in self.excludes.iter() {
            writeln!(f, "{}", dir.display())?;
        }

        if self.hide {
            f.write_str("Ignore hidden files")
        } else {
            f.write_str("Include hidden files")
        }
    }
}

fn canonicalize(paths: &mut Vec<PathBuf>) {
    paths.retain_mut(|dir| match fs_err::canonicalize(dir.clone()) {
        Ok(d) => {
            *dir = d;
            true
        }
        Err(e) => {
            tracing::warn!("{}", e);
            false
        }
    });
}
