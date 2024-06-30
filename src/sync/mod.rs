pub mod directory_traversal;

pub use directory_traversal::CopyJob;
use std::{fs, io};
use std::path::Path;

// Min number of bytes for it to be deemed worthwhile comparing the contents of a file in src and dest
const MIN_BYTES_FOR_SYNC: u64 = 1024;


pub fn sync(job: CopyJob) -> io::Result<u64> {
    if job.may_exist {
        merge(job)
    } else {
        copy(job)
    }
}

fn merge(job: CopyJob) -> io::Result<u64> {
    let dest_meta = match job.dest.symlink_metadata() {
        Ok(x) => x,
        // assume that dest doesn't exist - if false then we're just kicking the error can down the road
        Err(_) => return copy(job),
    };
    if dest_meta.is_dir() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("Found directory with conflicting name: {}", job.dest.display())))
    };
    // dest is either a symlink or a file
    if dest_meta.len() < MIN_BYTES_FOR_SYNC {
        fs::remove_file(&job.dest)?;
        copy(job)
    } else {
        todo!()
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
