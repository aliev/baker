use baker::{
    cli::{get_args, Args},
    config::{config_file_read, config_parse_content},
    error::{default_error_handler, BakerError, BakerResult},
    hooks::{confirm_hooks_execution, get_hooks, run_hook},
    ignore::ignore_file_read,
    processor::process_template,
    prompt::prompt_config_values,
    template::{
        FileSystemTemplateSourceProcessor, GitTemplateSourceProcessor, MiniJinjaTemplateRenderer,
        TemplateRenderer, TemplateSource, TemplateSourceProcessor,
    },
};

fn run(args: Args) -> BakerResult<()> {
    if let Some(project_template_type) = TemplateSource::from_string(&args.template) {
        let template_source: Box<dyn TemplateSourceProcessor> = match project_template_type {
            TemplateSource::Git(_) => Box::new(GitTemplateSourceProcessor::new()),
            TemplateSource::FileSystem(_) => Box::new(FileSystemTemplateSourceProcessor::new()),
        };
        let template_dir = template_source.process(&project_template_type)?;

        let mut execute_hooks = false;
        let (pre_hook, post_hook) = get_hooks(&template_dir);

        if pre_hook.exists() || post_hook.exists() {
            execute_hooks = confirm_hooks_execution(args.skip_hooks_check)?;
        }

        // Template processor
        let template_renderer: Box<dyn TemplateRenderer> =
            Box::new(MiniJinjaTemplateRenderer::new());

        // Processing the .bakerignore
        let ignored_set = ignore_file_read(&template_dir.join(".bakerignore"))?;

        // Processing the bakerfile.
        let config_file = template_dir.join("baker.json");
        let config_content = config_file_read(&config_file)?;

        println!("Loading configuration from: {}", &config_file.display());
        let config = config_parse_content(config_content, &template_renderer)?;
        let context = prompt_config_values(config)?;

        if execute_hooks && pre_hook.exists() {
            run_hook(&pre_hook, &context)?;
        }

        let output_dir = process_template(
            &template_dir,
            &args.output_dir,
            &context,
            &template_renderer,
            ignored_set,
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
