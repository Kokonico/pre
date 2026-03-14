mod args;
mod constants;
mod help;
mod helpers;
mod models;
mod rendering;
mod run;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    let config = match args::parse_args() {
        Some(c) => c,
        None => return,
    };

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);

    ctrlc::set_handler(move || {
        stop_clone.store(true, Ordering::SeqCst);
    })
    .expect("Failed to set Ctrl+C handler");

    let result = run::run(&config, &stop);

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
