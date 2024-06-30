pub mod directory_traversal;

pub use directory_traversal::CopyJob;
use std::{fs, io};
use std::path::Path;


pub fn sync(job: CopyJob) -> io::Result<u64> {
    if job.may_exist {
        todo!()
    } else {
        copy(job)
    }
}

// Handles a dumb copy. It is assumed dest does not exist
// # Errors
// 1. If the copy failed
// 1. If the file is a symlink and dest already exists
fn copy(job: CopyJob) -> io::Result<u64> {
    if job.symlink {
        copy_symlink(&job.src, &job.dest).and(Ok(0))
    } else {
        fs::copy(&job.src, &job.dest)
    }
}

// Copy (i.e. recreate) src at dest.
// The symlink's target will be completely unmodified, i.e.
// 1. If the target is an absolute path, dest will point to that path
// 1. If the target is a relative path, dest will also be relative (from dest's location). This can lead to broken symlinks.
// 
// # Errors
// 1. If src is not a symlink
// 1. If the new symlink could not be created (e.g. the dest already exists)
fn copy_symlink(src: &Path, dest: &Path) -> io::Result<()> {
    let target = fs::read_link(src)?;
    std::os::unix::fs::symlink(target, dest)
}
