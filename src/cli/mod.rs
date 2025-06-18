// Re-export all public items from the cli submodules
pub mod answers;
pub mod args;
pub mod hooks;
pub mod runner;

pub use args::get_args;
use clap::{Parser, ValueEnum};
pub use runner::run;
use std::path::PathBuf;

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

impl Args {
    pub fn should_skip_all_confirms(&self) -> bool {
        self.skip_confirms.contains(&SkipConfirm::All)
    }

    pub fn should_skip_overwrite_confirms(&self) -> bool {
        self.should_skip_all_confirms()
            || self.skip_confirms.contains(&SkipConfirm::Overwrite)
    }

    pub fn should_skip_hook_confirms(&self) -> bool {
        self.should_skip_all_confirms()
            || self.skip_confirms.contains(&SkipConfirm::Hooks)
    }
}
