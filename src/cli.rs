//! Command-line interface implementation for Baker.
//! Provides argument parsing and help text formatting using clap.

use clap::{error::ErrorKind, CommandFactory, Parser};
use std::path::PathBuf;

/// Command-line arguments structure for Baker.
#[derive(Parser, Debug)]
#[command(author, version, about = "Baker: fast and flexible project scaffolding tool", long_about = None)]
#[command(after_help = r#"Usage Examples:
    # Create a new project from a local template:
    $ baker ./path/to/template ./output

    # Create a new project from a git repository:
    $ baker https://github.com/user/template.git ./output

    # Force overwrite an existing output directory:
    $ baker -f ./template ./existing-dir

    # Enable verbose output:
    $ baker -v ./template ./output

Template Structure:
    template/
    ├── baker.json          # Template configuration
    ├── .bakerignore        # Files to ignore (optional)
    ├── hooks/              # Template hooks (optional)
    │   ├── pre_gen_project
    │   └── post_gen_project
    └── ... template files ..."#)]
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

    /// Get answers from stding
    /// For example:
    /// $ echo '{"question_key": "answer"}' | baker template out
    #[arg(short, long)]
    pub stdin: bool,

    /// Skip confirmation prompts when overwriting existing files.
    /// This will automatically overwrite any existing files in the output directory
    /// without asking for confirmation.
    #[arg(long)]
    pub skip_overwrite_check: bool,

    /// Skip confirmation prompts when executing hooks.
    /// This will automatically execute any pre/post hooks defined in the template
    /// without asking for confirmation first.
    #[arg(long)]
    pub skip_hooks_check: bool,
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
