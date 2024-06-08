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
    #[arg(short, long, default_value_t = 1, help = "How many worker threads to spawn")]
    threads: u8,
    #[arg(short, long, help = "Display speed stats during the copy")]
    stats: bool,
    #[arg(long, help = "Only copy files from src differing/missing in dest")]
    sync: bool,
}

enum Message {
    Copied(u64),
    Err(io::Error),
}

pub fn run(mut args: Cli) -> Result<(), Box<dyn Error>> {
    validate_args(&mut args)?;
    let pool = Arc::new(ThreadPool::new(args.threads.into()));
    let traversal_pool = Arc::downgrade(&pool);
    let (tx, rx) = mpsc::channel();
    let mut stdout = stdout().lock();
    let mut mb_copied = 0f64;
    let start = time::Instant::now();
    pool.run(move || begin_traversal(&args.src, &args.dest, traversal_pool, tx));
    loop {
        match rx.recv() {
            Ok(m) => match m {
                Message::Copied(b) => {
                    if args.stats {
                        mb_copied += (b as f64) / 1024f64.powi(2);
                        let elapsed = start.elapsed().as_secs_f64();
                        let speed = mb_copied / elapsed;
                        write!(stdout, "\rCopied {:.1}MB in {:.1}s = {:.1}MB/s", mb_copied, elapsed, speed)?;
                    }
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
    if !args.src.exists() {
        return Err("Source directory not accessible");
    }
    if !args.src.is_dir() {
        return Err("Source directory not a valid directory")
    }
    let dest_name = match args.src.file_name() {
        Some(s) => s,
        None => return Err("Can't copy from .. (yet)"),
    };
    if args.dest.exists() {
        args.dest = args.dest.join(dest_name);
    }
    if !args.dest.exists() {
        args.sync = false;
        return fs::create_dir(&args.dest).or(Err("Couldn't create destination directory"))
    }
    if !args.sync {
        return Err("Destination directory already exists. Consider --sync")
    }
    Ok(())
}

fn copy(job: CopyJob, sender: mpsc::Sender<Message>) {
    let res = if job.symlink {
        copy_symlink(&job.src, &job.dest).and(Ok(0))
    } else {
        fs::copy(&job.src, &job.dest)
    };
    sender.send(match res {
        Ok(b) => Message::Copied(b),
        Err(e) => Message::Err(e),
    }).unwrap();
}

// Copy (i.e. recreate) src at dest.
// The symlink's target will be completely unmodified, i.e.
// 1. If the target is an absolute path, dest will point to that path
// 1. If the target is a relative path, dest will also be relative (from dest's location). This can lead to broken symlinks.
// 
// # Errors
// 1. If src is not a symlink
// 1. If the new symlink could not be created
fn copy_symlink(src: &Path, dest: &Path) -> io::Result<()> {
    let target = fs::read_link(src)?;
    std::os::unix::fs::symlink(target, dest)
}

fn begin_traversal(src: &Path, dest: &Path, pool: Weak<ThreadPool>, sender: mpsc::Sender<Message>) {
    let traversal_iterator = match directory_traversal::traverse(src, dest) {
        Ok(x) => x,
        Err(e) => {
            sender.send(Message::Err(e)).unwrap();
            return
        },
    };
    for job in traversal_iterator {
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
