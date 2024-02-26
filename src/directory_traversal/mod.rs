use std::io;
use std::path::{Path, PathBuf};
use std::fs;

pub struct CopyJob {
    pub src: PathBuf,
    pub dest: PathBuf,
}

/// Returns an iterator over the subtree of src, yielding CopyJob(s)
/// and creating parent directories under dest as it goes. Returns an
/// io::Error if a directory cannot be read/created (e.g. permissions)
/// or if an intermittent IO fault is encountered.
///
/// # Parameters
/// src - an existing directory in which to begin traversal
/// dest - an existing directory in which to create any dir trees found within src
pub fn traverse(src: &Path, dest: &Path) -> CopyJobIterator {
    let state = State {
        src: src.to_path_buf(),
        dest: dest.to_path_buf(),
        src_entries: None,
    };
    CopyJobIterator {
        stack: vec![state],
        errored: false,
    }
}

struct State {
    src: PathBuf,
    dest: PathBuf,
    src_entries: Option<fs::ReadDir>,
}

pub struct CopyJobIterator {
    stack: Vec<State>,
    errored: bool,
}

impl CopyJobIterator {
    fn wrap_error(&mut self, err: io::Error) -> Option<io::Result<CopyJob>> {
        self.errored = true;
        Some(Err(err))
    }
}

impl Iterator for CopyJobIterator {
    type Item = io::Result<CopyJob>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.errored || self.stack.len() == 0 {
            return None
        }
        let state = self.stack.pop().unwrap();
        let mut src_entries = match state.src_entries {
            Some(entries) => entries,
            None => {
                match fs::read_dir(&state.src) {
                    Ok(entries) => entries,
                    Err(e) => return self.wrap_error(e),
                }
            },
        };
        while let Some(res) = src_entries.next() {
            let entry = match res {
                Ok(entry) => entry.path(),
                Err(e) => return self.wrap_error(e),
            };
            let dest = state.dest.join(entry.file_name().unwrap());
            if entry.is_dir() {
                if let Err(e) = fs::create_dir(&dest) {
                    return self.wrap_error(e)
                };
                self.stack.push(State {
                    src: entry,
                    dest,
                    src_entries: None
                });
            } else {
                let job = CopyJob {
                    src: entry,
                    dest,
                };
                self.stack.push(State {
                    src: state.src,
                    dest: state.dest,
                    src_entries: Some(src_entries),
                });
                return Some(Ok(job))
            }
        };
        return self.next()
    }
}