use baker::{
    error::{BakerError, BakerResult},
    template::{
        GithubTemplateSourceProcessor, LocalTemplateSourceProcessor, MiniJinjaTemplateProcessor,
        TemplateSource, TemplateSourceProcessor,
    },
};
use clap::Parser;
use globset::{Glob, GlobSet, GlobSetBuilder};
use log::{error, debug};
use std::{path::PathBuf, fs::read_to_string};

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

fn check_output_dir(output_dir: &PathBuf, force: bool) -> BakerResult<()> {
    if output_dir.exists() && !force {
        return Err(BakerError::ConfigError(format!(
            "Output directory already exists: {}. Use --force to overwrite",
            output_dir.display()
        )));
    }
    Ok(())
}

fn get_bakerignore(source_dir: &PathBuf) -> BakerResult<GlobSet> {
    let bakerignore_path = source_dir.join(".bakerignore");
    let mut builder = GlobSetBuilder::new();
    if let Ok(contents) = read_to_string(bakerignore_path) {
        for line in contents.lines() {
            builder.add(Glob::new(line).map_err(|e| {
                BakerError::BakerIgnoreError(format!(".bakerignore loading failed: {}", e))
            })?);
        }
    } else {
        debug!(".bakerignore does not exist")
    }
    let glob_set = builder.build().map_err(|e| {
        BakerError::BakerIgnoreError(format!(".bakerignore loading failed: {}", e))
    })?;

    Ok(glob_set)
}

fn run(args: Args) -> BakerResult<()> {
    if let Some(template_source) = TemplateSource::from_string(&args.template) {
        let template_source_processor: Box<dyn TemplateSourceProcessor> = match template_source {
            TemplateSource::GitHub(_) => Box::new(GithubTemplateSourceProcessor::new()),
            TemplateSource::LocalPath(_) => Box::new(LocalTemplateSourceProcessor::new()),
        };
        let template_processor = MiniJinjaTemplateProcessor::new();
        let template_dir = template_source_processor.process(template_source)?;
        let output_dir = &args.output_dir;
        check_output_dir(output_dir, args.force)?;

        // Processing the .bakerignore
        let bakerignore = get_bakerignore(&template_dir)?;
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
