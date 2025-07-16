pub mod answers;
pub mod hooks;
pub mod processor;
pub mod runner;

use crate::constants::{exit_codes, verbosity};
use clap::{error::ErrorKind, CommandFactory, Parser, ValueEnum};
/// Pre and post generation hook processing.
use log::LevelFilter;
use std::fmt::Display;
use std::path::PathBuf;

pub use runner::run;

// Help template for missing required arguments
const HELP_TEMPLATE: &str = r#"{about-section}
{usage-heading} {usage}

{all-args}
{after-help}
"#;

/// Get the appropriate log level from verbose count
pub fn get_log_level_from_verbose(verbose_count: u8) -> LevelFilter {
    match verbose_count {
        verbosity::OFF => LevelFilter::Error, // Default level when no -v flags
        verbosity::INFO => LevelFilter::Info, // -v
        verbosity::DEBUG => LevelFilter::Debug, // -vv
        verbosity::TRACE.. => LevelFilter::Trace, // -vvv and beyond
    }
}

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

impl Display for SkipConfirm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SkipConfirm::All => "all",
            SkipConfirm::Overwrite => "overwrite",
            SkipConfirm::Hooks => "hooks",
        };
        write!(f, "{s}")
    }
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
    ///
    /// Use multiple times to increase verbosity:
    ///
    /// * No flag: WARN level
    ///
    /// * `-v`: INFO level
    ///
    /// * `-vv`: DEBUG level
    ///
    /// * `-vvv`: TRACE level (maximum verbosity)
    ///
    /// Examples:
    ///
    /// > baker template output -v     # INFO level
    ///
    /// > baker template output -vv    # DEBUG level
    ///
    /// > baker template output -vvv   # TRACE level
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

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

    /// Show what would be done without actually executing any file operations
    /// In dry-run mode, Baker will process templates and show all operations
    /// that would be performed, but won't create directories, copy files, or write content
    #[arg(long = "dry-run")]
    pub dry_run: bool,
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
    Args::try_parse().unwrap_or_else(|e| {
        if e.kind() == ErrorKind::MissingRequiredArgument {
            Args::command().help_template(HELP_TEMPLATE).print_help().unwrap();
            std::process::exit(exit_codes::FAILURE);
        } else {
            e.exit();
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_log_level_from_verbose() {
        use crate::constants::verbosity;
        use log::LevelFilter;
        assert_eq!(get_log_level_from_verbose(verbosity::OFF), LevelFilter::Error);
        assert_eq!(get_log_level_from_verbose(verbosity::INFO), LevelFilter::Info);
        assert_eq!(get_log_level_from_verbose(verbosity::DEBUG), LevelFilter::Debug);
        assert_eq!(get_log_level_from_verbose(verbosity::TRACE), LevelFilter::Trace);
        // Test for values above TRACE
        assert_eq!(get_log_level_from_verbose(verbosity::TRACE + 1), LevelFilter::Trace);
    }

    #[test]
    fn test_args_parsing() {
        use super::Args;
        use clap::Parser;
        let args = Args::parse_from(["baker", "template_dir", "output_dir", "--force"]);
        assert_eq!(args.template, "template_dir");
        assert_eq!(args.output_dir, std::path::PathBuf::from("output_dir"));
        assert!(args.force);
    }

    #[test]
    fn test_skip_confirm_display() {
        use super::SkipConfirm;
        assert_eq!(format!("{}", SkipConfirm::All), "all");
        assert_eq!(format!("{}", SkipConfirm::Overwrite), "overwrite");
        assert_eq!(format!("{}", SkipConfirm::Hooks), "hooks");
    }

    #[test]
    fn test_args_parsing_with_all_flags() {
        use super::Args;
        use super::SkipConfirm;
        use clap::Parser;
        let args = Args::parse_from([
            "baker",
            "template_dir",
            "output_dir",
            "--force",
            "-vvv",
            "--answers",
            "{\"name\":\"John\"}",
            "--skip-confirms",
            "all,overwrite",
            "--non-interactive",
            "--dry-run",
        ]);
        assert_eq!(args.template, "template_dir");
        assert_eq!(args.output_dir, std::path::PathBuf::from("output_dir"));
        assert!(args.force);
        assert_eq!(args.verbose, 3);
        assert_eq!(args.answers, Some("{\"name\":\"John\"}".to_string()));
        assert!(args.skip_confirms.contains(&SkipConfirm::All));
        assert!(args.skip_confirms.contains(&SkipConfirm::Overwrite));
        assert!(args.non_interactive);
        assert!(args.dry_run);
    }
}
