use baker::{
    error::{BakerError, BakerResult},
    template::{
        GithubTemplateSourceProcessor, LocalTemplateSourceProcessor, MiniJinjaTemplateProcessor,
        TemplateSource, TemplateSourceProcessor,
    },
};
use clap::Parser;
use globset::{Glob, GlobSet, GlobSetBuilder};
use indexmap::IndexMap;
use log::{debug, error};
use std::fs;
use std::{fs::read_to_string, path::PathBuf};

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

// Fetches the .bakerignore file from template directory and returns GlobSet object.
fn get_bakerignore(bakerignore_path: &PathBuf) -> BakerResult<GlobSet> {
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
    let glob_set = builder
        .build()
        .map_err(|e| BakerError::BakerIgnoreError(format!(".bakerignore loading failed: {}", e)))?;

    Ok(glob_set)
}

// Fethes the baker.json from template directory and returns the ordered map.
fn get_bakerfile(bakerfile_path: &PathBuf) -> BakerResult<IndexMap<String, serde_json::Value>> {
    if !bakerfile_path.exists() || !bakerfile_path.is_file() {
        return Err(BakerError::ConfigError(format!(
            "Invalid configuration path: {}",
            bakerfile_path.display()
        )));
    }

    let content = fs::read_to_string(&bakerfile_path).map_err(BakerError::IoError)?;
    let map: IndexMap<String, serde_json::Value> =
        serde_json::from_str(&content).map_err(|e| BakerError::ConfigError(e.to_string()))?;
    Ok(map)
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
        let template_processor = MiniJinjaTemplateProcessor::new();

        // Processing the .bakerignore
        let bakerignore = get_bakerignore(&template_dir.join(".bakerignore"))?;

        // Processing the baker.json
        let bakerfile_content = get_bakerfile(&template_dir.join("baker.json"))?;
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
