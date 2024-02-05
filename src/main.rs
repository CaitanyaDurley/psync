use std::process;
use psync::{Cli, Parser, run};

fn main() {
    let args = Cli::parse();
    if let Err(e) = run(args) {
        eprintln!("Runtime error: {e}");
        process::exit(1);
    }
}
