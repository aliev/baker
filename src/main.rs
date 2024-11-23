use baker::{args::run, args::Args};
use clap::Parser;
use log::error;

pub fn init_logger(verbose: bool) {
    env_logger::Builder::new()
        .filter_level(if verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();
}

fn main() {
    let args = Args::parse();
    init_logger(args.verbose);

    if let Err(err) = run(args) {
        error!("Error: {}", err);
        std::process::exit(1);
    }
}
