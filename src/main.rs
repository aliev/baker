use baker::{
    bakerfile::read_bakerfile,
    bakerignore::read_bakerignore,
    config::parse_config,
    error::{BakerError, BakerResult},
    hooks::{confirm_hooks_execution, get_hooks, run_hook},
    processor::process_template,
    prompt::prompt_for_values,
    render::{MiniJinjaTemplateRenderer, TemplateRenderer},
    template::{
        FileSystemTemplateSourceProcessor, GitTemplateSourceProcessor, TemplateSource,
        TemplateSourceProcessor,
    },
};
use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Template argument
    #[arg(value_name = "TEMPLATE")]
    template: String,

    /// Output directory path
    #[arg(value_name = "OUTPUT_DIR")]
    output_dir: PathBuf, // Keep as PathBuf since we need to own it

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

fn get_output_dir<P: AsRef<Path>>(output_dir: P, force: bool) -> BakerResult<PathBuf> {
    let output_dir = output_dir.as_ref();
    if output_dir.exists() && !force {
        return Err(BakerError::ConfigError(format!(
            "output directory already exists: {}. Use --force to overwrite",
            output_dir.display()
        )));
    }
    Ok(output_dir.to_path_buf())
}

fn run(args: Args) -> BakerResult<()> {
    if let Some(template_source) = TemplateSource::from_string(&args.template) {
        let template_source_processor: Box<dyn TemplateSourceProcessor> = match template_source {
            TemplateSource::Git(_) => Box::new(GitTemplateSourceProcessor::new()),
            TemplateSource::FileSystem(_) => Box::new(FileSystemTemplateSourceProcessor::new()),
        };
        let template_dir = template_source_processor.process(&template_source)?;
        let output_dir = get_output_dir(args.output_dir, args.force)?;

        let mut execute_hooks = false;
        let (pre_hook, post_hook) = get_hooks(&template_dir);

        if pre_hook.exists() || post_hook.exists() {
            execute_hooks = confirm_hooks_execution(args.skip_hooks_check)?;
        }

        // Template processor
        let template_processor: Box<dyn TemplateRenderer> =
            Box::new(MiniJinjaTemplateRenderer::new());

        // Processing the .bakerignore
        let bakerignore = read_bakerignore(&template_dir.join(".bakerignore"))?;

        // Processing the bakerfile.
        let bakerfile = template_dir.join("baker.json");
        let bakerfile_content = read_bakerfile(&bakerfile)?;

        println!("Loading configuration from: {}", &bakerfile.display());
        let config = parse_config(bakerfile_content, &template_processor)?;
        let context = prompt_for_values(config)?;

        if execute_hooks && pre_hook.exists() {
            run_hook(&pre_hook, &context)?;
        }

        process_template(
            &template_dir,
            &output_dir,
            &context,
            &template_processor,
            bakerignore,
        )?;

        if execute_hooks && post_hook.exists() {
            run_hook(&post_hook, &context)?;
        }

        println!(
            "Template generation completed successfully in directory {}!",
            output_dir.display()
        );
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
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
