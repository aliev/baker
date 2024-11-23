use log::error;
use clap::Parser;
use baker::{args::Args, logger::init_logger, run::run};

fn main() {
    let args = Args::parse();
    init_logger(args.verbose);

    if let Err(err) = run(args) {
        error!("Error: {}", err);
        std::process::exit(1);
    }
}
