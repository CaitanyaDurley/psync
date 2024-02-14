mod thread_pool;
pub use clap::Parser;
use std::error::Error;
use std::fs;
use std::io;
use std::io::{stdout, Write};
use std::time;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
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
    ToCopy(CopyJob),
    Copied(u64),
    Err(io::Error),
}

pub fn run(mut args: Cli) -> Result<(), Box<dyn Error>> {
    validate_args(&mut args)?;
    let pool = ThreadPool::new(args.threads.into());
    let (tx, rx) = mpsc::channel();
    let sender = Arc::new(tx);
    let mut stdout = stdout().lock();
    let mut total_copied: u64 = 0;
    let mut idle;
    let start = time::Instant::now();
    let traversal_sender = Arc::clone(&sender);
    pool.run(move || begin_traversal(&args.src, &args.dest, traversal_sender));
    loop {
        // idle is true iff traversal is done and there are no currently running copy jobs
        idle = Arc::strong_count(&sender) == 1;
        match rx.try_recv() {
            Ok(m) => match m {
                Message::ToCopy(job) => {
                    let copy_sender = Arc::clone(&sender);
                    pool.run(move || copy(job, copy_sender))
                },
                Message::Copied(b) => {
                    total_copied += b;
                    let mb_copied = (total_copied as f64) / 1024f64.powi(2);
                    let elapsed = start.elapsed().as_secs_f64();
                    let speed = mb_copied / elapsed;
                    write!(stdout, "\rCopied {:.1}MB in {:.1}s = {:.1}MB/s", mb_copied, elapsed, speed)?;
                },
                Message::Err(e) => return Err(Box::new(e)),
            },
            Err(_) => if idle {
                // there's no messages on the channel and no more are coming
                break
            },
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

fn copy(job: CopyJob, sender: Arc<mpsc::Sender<Message>>) {
    sender.send(match fs::copy(&job.src, &job.dest) {
        Ok(b) => Message::Copied(b),
        Err(e) => Message::Err(e),
    }).unwrap();
}

fn begin_traversal(src: &Path, dest: &Path, sender: Arc<mpsc::Sender<Message>>) {
    for job in directory_traversal::traverse(src, dest) {
        sender.send(match job {
            Ok(job) => Message::ToCopy(job),
            Err(e) => Message::Err(e),
        }).unwrap();
    }
}

mod directory_traversal {
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
}
