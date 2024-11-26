use baker::{
    bakerfile::read_bakerfile,
    bakerignore::read_bakerignore,
    cli::{get_args, Args},
    config::parse_config,
    error::{default_error_handler, BakerError, BakerResult},
    hooks::{confirm_hooks_execution, get_hooks, run_hook},
    processor::process_template,
    prompt::prompt_for_values,
    render::{MiniJinjaTemplateRenderer, TemplateRenderer},
    template::{
        FileSystemTemplateSourceProcessor, GitTemplateSourceProcessor, TemplateSource,
        TemplateSourceProcessor,
    },
};

fn run(args: Args) -> BakerResult<()> {
    if let Some(template_source) = TemplateSource::from_string(&args.template) {
        let template_source_processor: Box<dyn TemplateSourceProcessor> = match template_source {
            TemplateSource::Git(_) => Box::new(GitTemplateSourceProcessor::new()),
            TemplateSource::FileSystem(_) => Box::new(FileSystemTemplateSourceProcessor::new()),
        };
        let template_dir = template_source_processor.process(&template_source)?;

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

        let output_dir = process_template(
            &template_dir,
            &args.output_dir,
            &context,
            &template_processor,
            bakerignore,
            args.force,
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
    let args = get_args();

    env_logger::Builder::new()
        .filter_level(if args.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();

    if let Err(err) = run(args) {
        default_error_handler(err);
    }
}
