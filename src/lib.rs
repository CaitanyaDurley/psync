mod thread_pool;
mod directory_traversal;

pub use clap::Parser;
use std::error::Error;
use std::fs;
use std::io;
use std::io::{stdout, Write};
use std::time;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Weak};
use thread_pool::ThreadPool;
use directory_traversal::CopyJob;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    src: PathBuf,
    dest: PathBuf,
    #[arg(short, long, default_value_t = 1)]
    threads: u8,
}

enum Message {
    Copied(u64),
    Err(io::Error),
}

pub fn run(mut args: Cli) -> Result<(), Box<dyn Error>> {
    validate_args(&mut args)?;
    let pool = Arc::new(ThreadPool::new(args.threads.into()));
    let (tx, rx) = mpsc::channel();
    let mut stdout = stdout().lock();
    let mut total_copied: u64 = 0;
    let start = time::Instant::now();
    let traversal_pool = Arc::downgrade(&pool);
    pool.run(move || begin_traversal(&args.src, &args.dest, traversal_pool, tx));
    loop {
        match rx.recv() {
            Ok(m) => match m {
                Message::Copied(b) => {
                    total_copied += b;
                    let mb_copied = (total_copied as f64) / 1024f64.powi(2);
                    let elapsed = start.elapsed().as_secs_f64();
                    let speed = mb_copied / elapsed;
                    write!(stdout, "\rCopied {:.1}MB in {:.1}s = {:.1}MB/s", mb_copied, elapsed, speed)?;
                },
                Message::Err(e) => return Err(Box::new(e)),
            },
            Err(_) => break,
        }
    }
    write!(stdout, "\n")?;
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

fn copy(job: CopyJob, sender: mpsc::Sender<Message>) {
    sender.send(match fs::copy(&job.src, &job.dest) {
        Ok(b) => Message::Copied(b),
        Err(e) => Message::Err(e),
    }).unwrap();
}

fn begin_traversal(src: &Path, dest: &Path, pool: Weak<ThreadPool>, sender: mpsc::Sender<Message>) {
    for job in directory_traversal::traverse(src, dest) {
        match job {
            Ok(job) => {
                let sender = sender.clone();
                let pool = pool.upgrade().unwrap();
                pool.run(move || copy(job, sender))
            },
            Err(e) => {
                sender.send(Message::Err(e)).unwrap();
                break
            },
        }
    }
}
