use baker::{
    cli::{get_args, Args},
    config::{load_config, parse_config},
    error::{default_error_handler, BakerError, BakerResult},
    hooks::{confirm_hooks_execution, get_hooks, run_hook},
    ignore::ignore_file_read,
    processor::process_template,
    prompt::prompt_config_values,
    template::{
        GitLoader, LocalLoader, MiniJinjaEngine, TemplateEngine, TemplateLoader, TemplateSource,
    },
};

fn run(args: Args) -> BakerResult<()> {
    if let Some(source) = TemplateSource::from_string(&args.template) {
        let loader: Box<dyn TemplateLoader> = match source {
            TemplateSource::Git(_) => Box::new(GitLoader::new()),
            TemplateSource::FileSystem(_) => Box::new(LocalLoader::new()),
        };
        let template_dir = loader.load(&source)?;

        let mut execute_hooks = false;
        let (pre_hook, post_hook) = get_hooks(&template_dir);

        if pre_hook.exists() || post_hook.exists() {
            execute_hooks = confirm_hooks_execution(args.skip_hooks_check)?;
        }

        // Template processor
        let engine: Box<dyn TemplateEngine> = Box::new(MiniJinjaEngine::new());

        // Processing the .bakerignore
        let ignored_set = ignore_file_read(&template_dir.join(".bakerignore"))?;

        // Processing the bakerfile.
        let config_file = template_dir.join("baker.json");
        let config_content = load_config(&config_file)?;

        println!("Loading configuration from: {}", &config_file.display());
        let config = parse_config(config_content, &engine)?;
        let context = prompt_config_values(config)?;

        if execute_hooks && pre_hook.exists() {
            run_hook(&pre_hook, &context)?;
        }

        let output_dir = process_template(
            &template_dir,
            &args.output_dir,
            &context,
            &engine,
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
