//! Baker's main application entry point and orchestration logic.
//! Handles command-line argument parsing, template processing flow,
//! and coordinates interactions between different modules.

use baker::{
    cli::{get_args, Args},
    config::{load_config, prompt_questions, Config, CONFIG_FILES},
    error::{default_error_handler, BakerError, BakerResult},
    hooks::{confirm_hooks_execution, get_hooks_dir, get_path_if_exists, run_hook},
    ignore::{parse_bakerignore_file, IGNORE_FILE},
    processor::process_template,
    template::{
        GitLoader, LocalLoader, MiniJinjaEngine, TemplateEngine, TemplateLoader, TemplateSource,
    },
};

/// Main application entry point.
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

/// Main application logic execution.
///
/// # Arguments
/// * `args` - Parsed command line arguments
///
/// # Returns
/// * `BakerResult<()>` - Success or error status of template processing
///
/// # Flow
/// 1. Initializes template loader based on source type
/// 2. Sets up hook execution if hooks exist
/// 3. Processes .bakerignore patterns
/// 4. Loads and parses configuration
/// 5. Prompts for template variables
/// 6. Executes pre-generation hooks
/// 7. Processes template files
/// 8. Executes post-generation hooks
fn run(args: Args) -> BakerResult<()> {
    if let Some(source) = TemplateSource::from_string(&args.template) {
        let loader: Box<dyn TemplateLoader> = match source {
            TemplateSource::Git(_) => Box::new(GitLoader::new()),
            TemplateSource::FileSystem(_) => Box::new(LocalLoader::new()),
        };
        let template_dir = loader.load(&source)?;
        // Load and parse configuration
        let config_content = load_config(&template_dir, &CONFIG_FILES)?;

        let mut execute_hooks = false;

        let (pre_hook, post_hook) = get_hooks_dir(&template_dir);

        if pre_hook.exists() || post_hook.exists() {
            execute_hooks = confirm_hooks_execution(
                args.skip_hooks_check,
                format!(
                    "WARNING: This template contains the following hooks that will execute commands on your system:\n{}{}{}",
                    get_path_if_exists(&post_hook),
                    get_path_if_exists(&pre_hook),
                    "Do you want to run these hooks?\n",
                ),
            )?;
        }

        // Template processor initialization
        let engine: Box<dyn TemplateEngine> = Box::new(MiniJinjaEngine::new());
        let config: Config = serde_yaml::from_str(&config_content).unwrap();
        let context = prompt_questions(config.questions, &engine)?;

        // Process ignore patterns
        let ignored_set = parse_bakerignore_file(&template_dir.join(IGNORE_FILE))?;

        // Execute pre-generation hook
        if execute_hooks && pre_hook.exists() {
            run_hook(&template_dir, &args.output_dir, &pre_hook, &context)?;
        }

        // Process template files
        let output_dir = process_template(
            &template_dir,
            &args.output_dir,
            &context,
            &engine,
            ignored_set,
            args.force,
        )?;

        // Execute post-generation hook
        if execute_hooks && post_hook.exists() {
            run_hook(&template_dir, &args.output_dir, &post_hook, &context)?;
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
