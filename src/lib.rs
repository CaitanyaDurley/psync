use std::error::Error;
use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use std::sync::mpsc;
use std::thread;
pub use clap::Parser;

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
    let to_copy = begin_traversal(args.src.clone());
    // let (done_tx, done_rx) = mpsc::channel();
    let mut traversing = true;
    let mut copying = true;
    let mut id = 0;
    while traversing || copying {
        match to_copy.try_recv() {
            Ok(path) => {
                let dest = args.dest.join(path.strip_prefix(&args.src)?);
                let job = CopyJob {
                    id,
                    src: path,
                    dest,
                    size: None,
                };
                copy(job);
                id += 1;
            },
            Err(mpsc::TryRecvError::Disconnected) => {traversing = false},
            Err(mpsc::TryRecvError::Empty) => (),
        }
    }
    Ok(())
}

fn copy(job: CopyJob) {
    todo!()
}

fn begin_traversal(src: PathBuf) -> mpsc::Receiver<PathBuf> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || traverse(&src, &tx));
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
