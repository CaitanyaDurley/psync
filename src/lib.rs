mod thread_pool;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use std::sync::mpsc;
pub use clap::Parser;
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
    id: usize,
    src: PathBuf,
    dest: PathBuf,
    size: Option<usize>,
}

pub fn run(args: Cli) -> Result<(), Box<dyn Error>> {
    let pool = ThreadPool::new(args.threads.into());
    let to_copy = begin_traversal(&pool, args.src.clone());
    loop {
        match to_copy.recv() {
            Ok(path) => {
                let dest = args.dest.join(path.strip_prefix(&args.src)?);
                pool.run(move || copy(&path, &dest));
            },
            Err(_) => break,
        }
    }
    Ok(())
    // pool goes out of scope, and its Drop implementation joins all worker threads
}

fn copy(src: &Path, dest: &Path) {
    todo!()
}

fn begin_traversal(pool: &ThreadPool, src: PathBuf) -> mpsc::Receiver<PathBuf> {
    let (tx, rx) = mpsc::channel();
    // TODO - handle the result from traverse instead of unwrap
    pool.run(move || traverse(&src, &tx).unwrap());
    rx
}

fn traverse(dir: &Path, sender: &mpsc::Sender<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?.path();
        if entry.is_dir() {
            traverse(&entry, &sender)?;
        } else {
            if let Err(_) = sender.send(entry) {
                // the receiver on the main thread has disconnected
                panic!();
            }
        }
    }
    Ok(())
}
