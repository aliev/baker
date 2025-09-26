use crate::constants::{exit_codes, verbosity};
use clap::{error::ErrorKind, CommandFactory, Parser, ValueEnum};
use log::LevelFilter;
use std::fmt::Display;
use std::path::PathBuf;

const HELP_TEMPLATE: &str = r#"{about-section}
{usage-heading} {usage}

{all-args}
{after-help}
"#;

/// Skip confirmation prompts for specific stages.
#[derive(Debug, Clone, ValueEnum, Copy, PartialEq)]
#[value(rename_all = "lowercase")]
pub enum SkipConfirm {
    /// Skip every confirmation prompt.
    All,
    /// Skip file overwrite confirmations.
    Overwrite,
    /// Skip hook execution confirmations.
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

/// CLI arguments for Baker.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Template directory or Git repository.
    #[arg(value_name = "TEMPLATE")]
    pub template: String,

    /// Destination directory for generated files.
    #[arg(value_name = "OUTPUT_DIR")]
    pub output_dir: PathBuf,

    /// Force overwrite of an existing output directory.
    #[arg(short, long)]
    pub force: bool,

    /// Increase logging verbosity (`-v`, `-vv`, `-vvv`).
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Predefined answers as JSON string or `-` to read from stdin.
    #[arg(short, long)]
    pub answers: Option<String>,

    /// Confirmation prompts to skip (comma-separated).
    #[arg(long = "skip-confirms", value_delimiter = ',')]
    #[arg(value_enum)]
    pub skip_confirms: Vec<SkipConfirm>,

    /// Disable interactive prompts when answers are provided.
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// Preview actions without touching the filesystem.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

/// Parse command line arguments with custom handling for missing required inputs.
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

/// Map `-v` counts to the appropriate log level.
pub fn get_log_level_from_verbose(verbose_count: u8) -> LevelFilter {
    match verbose_count {
        verbosity::OFF => LevelFilter::Error,
        verbosity::INFO => LevelFilter::Info,
        verbosity::DEBUG => LevelFilter::Debug,
        verbosity::TRACE.. => LevelFilter::Trace,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_verbose_flags_to_log_filters() {
        use crate::constants::verbosity;
        assert_eq!(get_log_level_from_verbose(verbosity::OFF), LevelFilter::Error);
        assert_eq!(get_log_level_from_verbose(verbosity::INFO), LevelFilter::Info);
        assert_eq!(get_log_level_from_verbose(verbosity::DEBUG), LevelFilter::Debug);
        assert_eq!(get_log_level_from_verbose(verbosity::TRACE), LevelFilter::Trace);
        assert_eq!(get_log_level_from_verbose(verbosity::TRACE + 1), LevelFilter::Trace);
    }

    #[test]
    fn parses_minimal_args() {
        use clap::Parser;
        let args = Args::parse_from(["baker", "template_dir", "output_dir", "--force"]);
        assert_eq!(args.template, "template_dir");
        assert_eq!(args.output_dir, PathBuf::from("output_dir"));
        assert!(args.force);
    }

    #[test]
    fn display_skip_confirm_variants() {
        assert_eq!(SkipConfirm::All.to_string(), "all");
        assert_eq!(SkipConfirm::Overwrite.to_string(), "overwrite");
        assert_eq!(SkipConfirm::Hooks.to_string(), "hooks");
    }

    #[test]
    fn parses_full_feature_flags() {
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
        assert_eq!(args.output_dir, PathBuf::from("output_dir"));
        assert!(args.force);
        assert_eq!(args.verbose, 3);
        assert_eq!(args.answers, Some("{\"name\":\"John\"}".to_string()));
        assert!(args.skip_confirms.contains(&SkipConfirm::All));
        assert!(args.skip_confirms.contains(&SkipConfirm::Overwrite));
        assert!(args.non_interactive);
        assert!(args.dry_run);
    }
}
