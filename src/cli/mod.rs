pub mod answers;
pub mod processor;
pub mod runner;

use clap::{error::ErrorKind, CommandFactory, Parser, ValueEnum};
use std::path::PathBuf;

pub use runner::run;

#[derive(Debug, Clone, ValueEnum, Copy, PartialEq)]
#[value(rename_all = "lowercase")]
pub enum SkipConfirm {
    /// Skip all confirmation prompts
    All,
    /// Skip confirmation when overwriting existing files
    Overwrite,
    /// Skip confirmation when executing pre/post hooks
    Hooks,
}

/// Command-line arguments structure for Baker.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the template directory or git repository URL
    #[arg(value_name = "TEMPLATE")]
    pub template: String,

    /// Directory where the generated project will be created
    #[arg(value_name = "OUTPUT_DIR")]
    pub output_dir: PathBuf,

    /// Force overwrite of existing output directory
    #[arg(short, long)]
    pub force: bool,

    /// Enable verbose logging output
    #[arg(short, long)]
    pub verbose: bool,

    /// Specifies answers to use during template processing.
    ///
    /// Accepts either a JSON string or "-" to read from stdin.
    ///
    /// Format
    ///
    /// The input should be a JSON object with key-value pairs where:
    ///
    /// - keys are variable names from the template
    ///
    /// - values are the corresponding answers
    ///
    /// Arguments
    ///
    /// * If a string is provided, it should contain valid JSON
    ///
    /// * If "-" is provided, JSON will be read from stdin
    ///
    /// * If None, no predefined answers will be used
    ///
    /// Example
    ///
    /// Provide answers directly
    ///
    /// > baker template_dir output_dir --answers='{"name": "John", "age": 30}'
    ///
    /// Read answers from stdin
    ///
    /// > echo '{"name": "John"}' | baker template_dir output_dir --answers=-
    ///
    #[arg(short, long)]
    pub answers: Option<String>,

    /// Controls which confirmation prompts should be skipped during template processing.
    /// Multiple flags can be combined to skip different types of confirmations.
    ///
    /// Examples
    ///
    /// Skip all confirmation prompts
    ///
    /// > baker --skip-confirms=all
    ///
    /// Skip only file overwrite confirmations
    ///
    /// > baker --skip-confirms=overwrite
    ///
    /// Skip both overwrite and hooks confirmations
    ///
    /// > baker --skip-confirms=overwrite,hooks
    ///
    #[arg(long = "skip-confirms", value_delimiter = ',')]
    #[arg(value_enum)]
    pub skip_confirms: Vec<SkipConfirm>,

    /// Skip interactive prompts if answers are already provided
    /// Use with --answers to create a fully non-interactive workflow
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

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
