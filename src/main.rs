use baker::{
    cli::{get_args, get_log_level_from_verbose, run},
    error::default_error_handler,
};

fn main() {
    let args = get_args();
    let log_level = get_log_level_from_verbose(args.verbose);
    env_logger::Builder::new().filter_level(log_level).init();

    if let Err(err) = run(args) {
        default_error_handler(err);
    }
}
