use baker::{
    bakerfile::read_bakerfile,
    bakerignore::read_bakerignore,
    config::parse_config,
    error::{BakerError, BakerResult},
    prompt::prompt_for_values,
    render::{MiniJinjaTemplateProcessor, TemplateRenderer},
    template::{
        GithubTemplateSourceProcessor, LocalTemplateSourceProcessor, TemplateSource,
        TemplateSourceProcessor,
    },
};
use clap::Parser;
use log::{debug, error, info};
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

fn get_output_dir(output_dir: &PathBuf, force: bool) -> BakerResult<&PathBuf> {
    if output_dir.exists() && !force {
        return Err(BakerError::ConfigError(format!(
            "Output directory already exists: {}. Use --force to overwrite",
            output_dir.display()
        )));
    }
    Ok(output_dir)
}

fn run(args: Args) -> BakerResult<()> {
    if let Some(template_source) = TemplateSource::from_string(&args.template) {
        let template_source_processor: Box<dyn TemplateSourceProcessor> = match template_source {
            TemplateSource::GitHub(_) => Box::new(GithubTemplateSourceProcessor::new()),
            TemplateSource::LocalPath(_) => Box::new(LocalTemplateSourceProcessor::new()),
        };
        let template_dir = template_source_processor.process(template_source)?;
        let output_dir = get_output_dir(&args.output_dir, args.force)?;

        // Template processor
        let template_processor: Box<dyn TemplateRenderer> =
            Box::new(MiniJinjaTemplateProcessor::new());

        // Processing the .bakerignore
        let bakerignore = read_bakerignore(&template_dir.join(".bakerignore"))?;

        // Processing the bakerfile.
        let bakerfile = template_dir.join("baker.json");
        let bakerfile_content = read_bakerfile(&bakerfile)?;

        info!("Loading configuration from: {}", &bakerfile.display());
        let config = parse_config(bakerfile_content, &template_processor)?;

        debug!("Starting interactive configuration...");
        let final_context = prompt_for_values(config)?;
        debug!("Final configuration: {:#?}", final_context);
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
