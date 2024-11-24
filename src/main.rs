use baker::{
    error::{BakerError, BakerResult},
    template::{
        GithubTemplateSourceProcessor, LocalTemplateSourceProcessor, MiniJinjaTemplateProcessor,
        TemplateSource, TemplateSourceProcessor,
    },
};
use clap::Parser;
use log::error;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
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

fn run(args: Args) -> BakerResult<()> {
    if let Some(template_source) = TemplateSource::from_string(&args.template) {
        let template_source_processor: Box<dyn TemplateSourceProcessor> = match template_source {
            TemplateSource::GitHub(_) => Box::new(GithubTemplateSourceProcessor::new()),
            TemplateSource::LocalPath(_) => Box::new(LocalTemplateSourceProcessor::new()),
        };
        let template_processor = MiniJinjaTemplateProcessor::new();
        let source_dir = template_source_processor.process(template_source)?;
        let target_dir = args.output_dir;
    } else {
        return Err(BakerError::TemplateError(format!(
            "invalid template source: {}",
            args.template
        )));
    }
    Ok(())
}

fn main() {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(if args.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();

    if let Err(err) = run(args) {
        error!("Error: {}", err);
        std::process::exit(1);
    }
}
