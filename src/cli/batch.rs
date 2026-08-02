use std::{mem, path::PathBuf};

use crossfire::{MTx, mpmc::Array};

type Sender = MTx<Array<Vec<PathBuf>>>;

const CAPACITY: usize = 256;

pub struct BatchSender {
    files: Vec<PathBuf>,
    sender: Sender,
}

impl BatchSender {
    pub fn new(sender: Sender) -> Self {
        Self {
            files: Vec::with_capacity(CAPACITY),
            sender,
        }
    }

    pub fn push(&mut self, path: PathBuf) {
        if self.files.len() >= CAPACITY {
            self.flush();
        }
        self.files.push(path);
    }

    fn flush(&mut self) {
        if self.files.is_empty() {
            return;
        }
        let files = mem::take(&mut self.files);
        if let Err(e) = self.sender.send(files) {
            tracing::warn!("{e}")
        }
    }
}

impl Drop for BatchSender {
    fn drop(&mut self) {
        self.flush();
    }
}
