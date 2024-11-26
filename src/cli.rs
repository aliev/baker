use std::path::PathBuf;

use clap::{error::ErrorKind, CommandFactory, Parser};

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
    ├── baker.json           # Template configuration
    ├── .bakerignore        # Files to ignore (optional)
    ├── hooks/              # Template hooks (optional)
    │   ├── pre_gen_project
    │   └── post_gen_project
    └── ... template files ..."#)]
pub struct Args {
    /// Template argument
    #[arg(value_name = "TEMPLATE")]
    pub template: String,

    /// Output directory path
    #[arg(value_name = "OUTPUT_DIR")]
    pub output_dir: PathBuf, // Keep as PathBuf since we need to own it

    /// Force overwrite existing output directory
    #[arg(short, long)]
    pub force: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Skip hooks safety check
    #[arg(long)]
    pub skip_hooks_check: bool,
}

pub fn get_args() -> Args {
    let args = match Args::try_parse() {
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
    };

    args
}
