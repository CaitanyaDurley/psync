mod thread_pool;
pub use clap::Parser;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use thread_pool::ThreadPool;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    src: PathBuf,
    dest: PathBuf,
    #[arg(short, long, default_value_t = 1)]
    threads: u8,
}

struct CopyJob {
    src: PathBuf,
    dest: PathBuf,
    size: Option<usize>,
}

pub fn run(mut args: Cli) -> Result<(), Box<dyn Error>> {
    validate_args(&mut args)?;
    let pool = ThreadPool::new(args.threads.into());
    let to_copy = begin_traversal(&pool, args.src, args.dest);
    loop {
        match to_copy.recv() {
            Ok(job) => {
                pool.run(move || copy(job));
            }
            Err(_) => break,
        }
    }
    Ok(())
    // pool goes out of scope, and its Drop implementation joins all worker threads
}

fn validate_args(args: &mut Cli) -> Result<(), &str> {
    if args.threads == 0 {
        return Err("At least one thread needed");
    }
    if !Path::exists(&args.src) {
        return Err("Source directory not accessible");
    }
    let dest_name = match args.src.file_name() {
        Some(s) => s,
        None => return Err("Invalid source directory"),
    };
    if Path::exists(&args.dest) {
        args.dest = args.dest.join(dest_name);
    }
    if let Err(_) = fs::create_dir(&args.dest) {
        return Err("Error creating destination directory");
    }
    Ok(())
}

fn copy(job: CopyJob) {
    // TODO: handle errors better than unwrap
    // TODO - capture the returned total number of bytes copied
    fs::copy(&job.src, &job.dest).unwrap();
}

fn begin_traversal(pool: &ThreadPool, src: PathBuf, dest: PathBuf) -> mpsc::Receiver<CopyJob> {
    let (tx, rx) = mpsc::channel();
    // TODO - handle the result from traverse instead of unwrap
    pool.run(move || traverse(&src, &dest, &tx).unwrap());
    rx
}

/// Traverse a directory tree, sending file copy jobs down the sender
/// and creating parent directories under dest as you go
///
/// # Parameters
/// src - an existing directory in which to begin traversal
/// dest - an existing directory in which to create any dir trees found within src
/// sender - a channel for sending files which need copying
fn traverse(src: &Path, dest: &Path, sender: &mpsc::Sender<CopyJob>) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?.path();
        let dest = dest.join(entry.file_name().unwrap());
        if entry.is_dir() {
            fs::create_dir(&dest)?;
            traverse(&entry, &dest, &sender)?;
        } else {
            let job = CopyJob {
                src: entry.to_path_buf(),
                dest,
                size: None,
            };
            if let Err(_) = sender.send(job) {
                // the receiver on the main thread has disconnected
                panic!();
            }
        }
    }
    Ok(())
}
