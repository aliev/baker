use std::path::PathBuf;

use clap::Parser;

use crate::error::{BakerError, BakerResult};

#[derive(Debug)]
enum TemplateSource {
    LocalPath(PathBuf),
    GitHub(String),
}

impl TemplateSource {
    fn from_string(s: &str) -> Option<Self> {
        if s.starts_with("gh@") {
            Some(Self::GitHub(s[3..].to_string()))
        } else {
            let path = PathBuf::from(s);
            if path.exists() {
                Some(Self::LocalPath(path))
            } else {
                None
            }
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Template argument
    #[arg(value_name = "TEMPLATE")]
    template: String,

    /// Output directory path
    #[arg(value_name = "OUTPUT_DIR")]
    output_dir: PathBuf,

    /// Force overwrite existing output directory
    #[arg(short, long)]
    force: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Skip hooks safety check
    #[arg(long)]
    skip_hooks_check: bool,
}

pub fn run(args: Args) -> BakerResult<()> {
    let template_source = TemplateSource::from_string(&args.template).ok_or_else(|| {
        BakerError::TemplateError(format!(
            "Invalid template format or path does not exist: {}",
            args.template
        ))
    })?;
    match template_source {
        TemplateSource::LocalPath(path) => handle_local_template(path, &args)?,
        TemplateSource::GitHub(repo) => handle_github_template(repo, &args)?,
    }
    Ok(())
}

fn handle_local_template(path: PathBuf, args: &Args) -> BakerResult<()> {
    todo!("This feature is not implemented yet.")
}

fn handle_github_template(repo: String, args: &Args) -> BakerResult<()> {
    todo!("This feature is not implemented yet.")
}
