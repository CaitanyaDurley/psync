use std::error::Error;
use std::path::{Path, PathBuf};
pub use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    src: PathBuf,
    dest: PathBuf,
    #[arg(short, long, default_value_t = 1)]
    threads: u8,
}

pub fn run(args: Cli) -> Result<(), Box<dyn Error>> {
    Ok(())
}
