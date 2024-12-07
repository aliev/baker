//! Baker's main application entry point and orchestration logic.
//! Handles command-line argument parsing, template processing flow,
//! and coordinates interactions between different modules.

use baker::{
    cli::{get_args, Args},
    config::{load_config, Config, CONFIG_FILES},
    error::{default_error_handler, BakerError, BakerResult},
    hooks::{get_hooks, get_path_if_exists, run_hook},
    ignore::{parse_bakerignore_file, IGNORE_FILE},
    processor::{ensure_output_dir, process_entry},
    prompt::{
        parse_questions, prompt_boolean, prompt_confirm_hooks_execution,
        prompt_multiple_choice, prompt_single_choice, prompt_string, QuestionType,
    },
    template::{
        GitLoader, LocalLoader, MiniJinjaEngine, TemplateEngine, TemplateLoader,
        TemplateSource,
    },
};
use walkdir::WalkDir;

/// Main application entry point.
fn main() {
    let args = get_args();

    // Logger configuration
    env_logger::Builder::new()
        .filter_level(if args.verbose {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Off
        })
        .init();

    if let Err(err) = run(args) {
        default_error_handler(err);
    }
}

fn parse_default_context(
    key: String,
    default_value: serde_json::Value,
    context: &String,
) -> BakerResult<(String, serde_json::Value)> {
    let parsed: serde_json::Value = serde_json::from_str(context).map_err(|e| {
        BakerError::TemplateError(format!("Failed to parse context as JSON: {}", e))
    })?;

    let value = parsed.get(&key);
    let result_value = if value.is_some() {
        value.cloned().unwrap_or(serde_json::Value::Null)
    } else if !default_value.is_null() {
        default_value.clone()
    } else {
        serde_json::Value::Null
    };

    Ok((key, result_value))
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
        let output_dir = ensure_output_dir(args.output_dir, args.force)?;
        let loader: Box<dyn TemplateLoader> = match source {
            TemplateSource::Git(_) => Box::new(GitLoader::new()),
            TemplateSource::FileSystem(_) => Box::new(LocalLoader::new()),
        };
        let template_dir = loader.load(&source)?;
        // Load and parse configuration
        let config_content = load_config(&template_dir, &CONFIG_FILES)?;

        let mut execute_hooks = false;

        let (pre_hook, post_hook) = get_hooks(&template_dir);

        if pre_hook.exists() || post_hook.exists() {
            execute_hooks = prompt_confirm_hooks_execution(
                args.skip_hooks_check,
                format!(
                    "WARNING: This template contains the following hooks that will execute commands on your system:\n{}{}{}",
                    get_path_if_exists(&post_hook),
                    get_path_if_exists(&pre_hook),
                    "Do you want to run these hooks?",
                ),
            )?;
        }

        // Template processor initialization
        let engine: Box<dyn TemplateEngine> = Box::new(MiniJinjaEngine::new());
        // TODO: map_err
        let config: Config = serde_yaml::from_str(&config_content).unwrap();

        let context = parse_questions(
            config.items,
            &*engine,
            |prompt_rendered, key, question, default_value, question_type| {
                match question_type {
                    QuestionType::MultipleChoice => {
                        // when
                        // type: str
                        // choices: ...
                        // multiselect: true
                        if args.context.is_empty() {
                            prompt_multiple_choice(prompt_rendered, key, question)
                        } else {
                            parse_default_context(key, default_value, &args.context)
                        }
                    }
                    QuestionType::SingleChoice => {
                        // when
                        // type: str
                        // choices: ...
                        // multiselect: false
                        if args.context.is_empty() {
                            prompt_single_choice(
                                prompt_rendered,
                                key,
                                question,
                                default_value,
                            )
                        } else {
                            parse_default_context(key, default_value, &args.context)
                        }
                    }
                    QuestionType::YesNo => {
                        // when
                        // type: bool
                        if args.context.is_empty() {
                            prompt_boolean(prompt_rendered, key, default_value)
                        } else {
                            parse_default_context(key, default_value, &args.context)
                        }
                    }
                    QuestionType::Text => {
                        // when
                        // type: str
                        if args.context.is_empty() {
                            prompt_string(prompt_rendered, key, question, default_value)
                        } else {
                            parse_default_context(key, default_value, &args.context)
                        }
                    }
                }
            },
        )?;
        // Process ignore patterns
        let ignored_set = parse_bakerignore_file(template_dir.join(IGNORE_FILE))?;

        // Execute pre-generation hook
        if execute_hooks && pre_hook.exists() {
            run_hook(&template_dir, &output_dir, &pre_hook, &context)?;
        }

        // Process template files
        for entry in WalkDir::new(&template_dir) {
            if let Err(e) = process_entry(
                entry,
                &template_dir,
                &output_dir,
                &context,
                &*engine,
                &ignored_set,
                args.overwrite,
            ) {
                match e {
                    BakerError::TemplateError(msg) => {
                        log::warn!("Template processing failed: {}", msg)
                    }
                    BakerError::IoError(e) => log::error!("IO operation failed: {}", e),
                    _ => log::error!("Operation failed: {}", e),
                }
            }
        }

        // Execute post-generation hook
        if execute_hooks && post_hook.exists() {
            run_hook(&template_dir, &output_dir, &post_hook, &context)?;
        }

        println!(
            "Template generation completed successfully in {}.",
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
