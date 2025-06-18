use crate::cli::Args;
use clap::{error::ErrorKind, CommandFactory, Parser};

/// Parses command line arguments and returns the Args structure.
///
/// # Returns
/// * `Args` - Parsed command line arguments
///
/// # Exits
/// * With status code 1 if required arguments are missing
/// * With clap's default error handling for other argument errors
pub fn get_args() -> Args {
    match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            if e.kind() == ErrorKind::MissingRequiredArgument {
                Args::command()
                    .help_template(
                        r#"{about-section}
{usage-heading} {usage}

{all-args}
{after-help}
"#,
                    )
                    .print_help()
                    .unwrap();
                std::process::exit(1);
            } else {
                e.exit();
            }
        }
    }
}
