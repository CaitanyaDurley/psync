use std::io;
use std::path::{Path, PathBuf};
use std::fs;

pub struct CopyJob {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub symlink: bool,
}

/// Returns a `CopyJobIterator` over the subtree of src.
///
/// # Parameters
/// src - an existing directory in which to begin traversal
/// dest - an existing directory in which to create any dir trees found within src
/// 
/// # Errors
/// If src does not exist/is not a directory/cannot be opened
pub fn traverse(src: &Path, dest: &Path) -> io::Result<CopyJobIterator> {
    let state = State {
        src_entries: fs::read_dir(src)?,
        dest: dest.to_path_buf(),
    };
    Ok(CopyJobIterator {
        stack: vec![state],
        errored: false,
    })
}

struct State {
    src_entries: fs::ReadDir,
    dest: PathBuf,
}

// The iterator returned by `traverse`
// Yields `CopyJob`(s) and creates directories under dest as src is traversed
// It is guaranteed that the parent directory of a `CopyJob` returned by this iterator will exist
//
// # Errors
// 1. If a subdirectory cannot be read/created (e.g. permissions)
// 1. If an intermittent IO fault is encountered
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
        let mut src_entries = state.src_entries;
        while let Some(res) = src_entries.next() {
            let entry = match res {
                Ok(entry) => entry,
                Err(e) => return self.wrap_error(e),
            };
            let dest = state.dest.join(entry.file_name());
            let entry_type = match entry.file_type() {
                Ok(x) => x,
                Err(e) => return self.wrap_error(e),
            };
            if entry_type.is_dir() {
                if let Err(e) = fs::create_dir(&dest) {
                    return self.wrap_error(e)
                };
                let next_entries = match fs::read_dir(entry.path()) {
                    Ok(x) => x,
                    Err(e) => return self.wrap_error(e),
                };
                self.stack.push(State {
                    src_entries: next_entries,
                    dest,
                });
            } else {
                // entry is either a file or a symlink
                let job = CopyJob {
                    src: entry.path(),
                    dest,
                    symlink: entry_type.is_symlink(),
                };
                self.stack.push(State {
                    src_entries,
                    dest: state.dest,
                });
                return Some(Ok(job))
            }
        };
        return self.next()
    }
}