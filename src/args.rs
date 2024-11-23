use std::path::PathBuf;

use clap::Parser;

use crate::{
    error::{BakerError, BakerResult},
    template::{
        GithubTemplateProcessor, LocalTemplateProcessor, TemplateSource, TemplateSourceProcessor,
    },
};

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
    if let Some(template_source) = TemplateSource::from_string(&args.template) {
        let processor: Box<dyn TemplateSourceProcessor> = match template_source {
            TemplateSource::GitHub(_) => Box::new(GithubTemplateProcessor::new()),
            TemplateSource::LocalPath(_) => Box::new(LocalTemplateProcessor::new()),
        };
        processor.process(template_source)?;
    } else {
        return Err(BakerError::TemplateError(format!(
            "invalid template source: {}",
            args.template
        )));
    }
    Ok(())
}
