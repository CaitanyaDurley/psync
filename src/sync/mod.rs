pub mod directory_traversal;

pub use directory_traversal::CopyJob;
use std::os::unix::fs::MetadataExt;
use std::{fs, io};
use std::path::Path;


pub fn sync(job: CopyJob) -> io::Result<u64> {
    match DestState::get(&job)? {
        DestState::InSync => return Ok(0),
        DestState::OutOfSync => fs::remove_file(&job.dest)?,
        DestState::Missing => (),
        DestState::IsDirectory => return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("Found directory with conflicting name: {}", job.dest.display()))),
    };
    copy(&job)
}

enum DestState {
    InSync,
    OutOfSync,
    Missing,
    IsDirectory,
}

impl DestState {
    fn get(job: &CopyJob) -> io::Result<Self> {
        if !job.may_exist {
            return Ok(Self::Missing)
        }
        let src_meta = fs::symlink_metadata(&job.src)?;
        let dest_meta = match fs::symlink_metadata(&job.dest) {
            Ok(m) => m,
            // assume that dest doesn't exist - if false then we're just kicking the error can down the road
            Err(_) => return Ok(Self::Missing),
        };
        if dest_meta.is_dir() {
            return Ok(Self::IsDirectory)
        }
        if src_meta.modified()? == dest_meta.modified()? && src_meta.size() == dest_meta.size() {
            Ok(Self::InSync)
        } else {
            Ok(Self::OutOfSync)
        }
    }
}

// Handles a dumb copy. It is assumed dest does not exist
// # Errors
// 1. If the copy failed
// 1. If the file is a symlink and dest already exists
// 1. If dest exists and is a directory
fn copy(job: &CopyJob) -> io::Result<u64> {
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
