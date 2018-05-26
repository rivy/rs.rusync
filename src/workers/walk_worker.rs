use std::fs;
use std::fs::DirEntry;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use entry::Entry;
use fsops;
use progress::Progress;

pub struct WalkWorker {
    entry_output: Sender<Entry>,
    progress_output: Sender<Progress>,
    source: PathBuf,
}

impl WalkWorker {
    pub fn new(
        source: &Path,
        entry_output: Sender<Entry>,
        progress_output: Sender<Progress>,
    ) -> WalkWorker {
        WalkWorker {
            entry_output,
            progress_output,
            source: source.to_path_buf(),
        }
    }

    fn walk(&self) -> io::Result<()> {
        let mut num_files = 0;
        let mut total_size = 0;
        let mut subdirs: Vec<PathBuf> = vec![self.source.to_path_buf()];
        while !subdirs.is_empty() {
            let subdir = subdirs.pop().unwrap();
            for entry in fs::read_dir(subdir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    subdirs.push(path);
                } else {
                    let meta = self.process_file(&entry)?;
                    num_files += 1;
                    total_size += meta.len();
                    let sent = self.progress_output.send(Progress::Todo {
                        num_files,
                        total_size: total_size as usize,
                    });
                    if sent.is_err() {
                        return Err(fsops::to_io_error(&"stats output chan is closed".to_string()));
                    }
                }
            }
        }
        Ok(())
    }

    fn process_file(&self, entry: &DirEntry) -> io::Result<fs::Metadata> {
        let rel_path = fsops::get_rel_path(&entry.path(), &self.source)?;
        let parent_rel_path = rel_path.parent();
        if parent_rel_path.is_none() {
            return Err(fsops::to_io_error(&format!(
                "Could not get parent path of {}",
                rel_path.to_string_lossy()
            )));
        }

        let desc = rel_path.to_string_lossy();
        let src_entry = Entry::new(&desc, &entry.path());
        let metadata = src_entry.metadata().unwrap().clone();
        let sent = self.entry_output.send(src_entry);
        if sent.is_err() {
            return Err(fsops::to_io_error(&"entry output chan is closed".to_string()));
        }
        Ok(metadata)
    }

    pub fn start(&self) {
        let outcome = &self.walk();
        if outcome.is_err() {
            // Send err to output
        }
    }
}