pub mod directory_traversal;

pub use directory_traversal::CopyJob;
use std::{fs, io};
use std::io::Read;
use std::path::Path;
use std::collections::HashMap;

// Block size in bytes to compare src and dest
const BLOCK_SIZE: usize = 1024;
// Min number of bytes for it to be deemed worthwhile comparing the contents of a file in src and dest
const MIN_BYTES_FOR_SYNC: usize = 10 * BLOCK_SIZE;


pub fn sync(job: CopyJob) -> io::Result<u64> {
    if job.may_exist {
        merge_unchecked(job)
    } else {
        copy(job)
    }
}


// Entrypoint for a merge job. Calls copy if:
// 1. dest doesn't exist
// 1. dest is smaller than MIN_BYTES_FOR_SYNC (deletes dest first)
//. # Errors
// 1. If dest exists and is a dir
// 1. If the metadata of dest could not be retrieved
fn merge_unchecked(job: CopyJob) -> io::Result<u64> {
    let dest_meta = match job.dest.symlink_metadata() {
        Ok(x) => x,
        // assume that dest doesn't exist - if false then we're just kicking the error can down the road
        Err(_) => return copy(job),
    };
    if dest_meta.is_dir() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("Found directory with conflicting name: {}", job.dest.display())))
    };
    // dest is either a symlink or a file
    if dest_meta.len() < MIN_BYTES_FOR_SYNC as u64 {
        fs::remove_file(&job.dest)?;
        copy(job)
    } else {
        merge_checked(&job.src, &job.dest)
    }
}

// Handles a dumb copy. It is assumed dest does not exist
// # Errors
// 1. If the copy failed
// 1. If the file is a symlink and dest already exists
// 1. If dest exists and is a directory
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


// ********* rsync style logic *********

// Handles a merge of src into dest. Assumes:
// 1. src and dest exist
// 1. src and dest are files (not symlinks)
fn merge_checked(src: &Path, dest: &Path) -> io::Result<u64> {
    let dest_block_lookup = build_dest_block_lookup(dest)?;
    Ok(0)
}

fn build_dest_block_lookup(dest: &Path) -> io::Result<HashMap<[u8; BLOCK_SIZE], usize>> {
    let mut dest_file = fs::File::open(dest)?;
    let mut buf = [0; BLOCK_SIZE];
    let mut byte_counter = 0;
    let mut block_counter: usize = 0;
    let mut possible_eof = false;
    let mut block_lookup = HashMap::new();
    loop {
        let res = dest_file.read(&mut buf[byte_counter..]);
        if let Err(e) = res {
            match e.kind() {
                io::ErrorKind::Interrupted => continue,
                _ => return Err(e)
            }
        };
        let res = res.unwrap();
        if possible_eof && res == 0 {
            // two successive Ok(0) => EOF
            break
        }
        possible_eof = res == 0;
        byte_counter += res;
        if byte_counter == BLOCK_SIZE {
            block_lookup.insert(buf, block_counter);
            byte_counter = 0;
            block_counter += 1;
        }
    }
    Ok(block_lookup)
}