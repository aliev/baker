//! Baker's main application entry point and orchestration logic.
//! Handles command-line argument parsing, template processing flow,
//! and coordinates interactions between different modules.

use baker::{
    cli::{get_args, Args},
    config::{load_config, Config, ConfigItemType, CONFIG_FILES},
    error::{default_error_handler, BakerError, BakerResult},
    hooks::{get_hooks, get_path_if_exists, run_hook},
    ignore::{parse_bakerignore_file, IGNORE_FILE},
    parser::{get_context_value, get_question_value, QuestionType},
    processor::{ensure_output_dir, process_entry},
    prompt::prompt_confirm_hooks_execution,
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

        // Trying to local context from --context
        // If it fails it returns null Value.
        let parsed = get_context_value(args.context)?;

        let mut answers = serde_json::Map::new();

        for (key, item) in config.items {
            let current_context = serde_json::Value::Object(answers.clone());

            // Sometimes "help" contain the value with the template strings.
            // This function renders it and returns rendered value.
            let help_rendered =
                engine.render(&item.help, &current_context).unwrap_or(item.help.clone());

            match item.item_type {
                ConfigItemType::Str => {
                    let (key, value) = if !item.choices.is_empty() {
                        if item.multiselect {
                            get_question_value(
                                help_rendered,
                                key,
                                item,
                                serde_json::Value::Null,
                                QuestionType::MultipleChoice,
                                &parsed,
                            )?
                        } else {
                            let default_value = if let Some(default_value) = &item.default
                            {
                                if let Some(default_str) = default_value.as_str() {
                                    item.choices
                                        .iter()
                                        .position(|choice| choice == default_str)
                                        .unwrap_or(0)
                                } else {
                                    0
                                }
                            } else {
                                0
                            };
                            get_question_value(
                                help_rendered,
                                key,
                                item,
                                serde_json::Value::Number(default_value.into()),
                                QuestionType::SingleChoice,
                                &parsed,
                            )?
                        }
                    } else {
                        let default_value = if let Some(default_value) = &item.default {
                            if let Some(s) = default_value.as_str() {
                                engine.render(s, &current_context).unwrap_or_default()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };
                        get_question_value(
                            help_rendered,
                            key,
                            item,
                            serde_json::Value::String(default_value),
                            QuestionType::Text,
                            &parsed,
                        )?
                    };
                    answers.insert(key, value);
                }
                ConfigItemType::Bool => {
                    let default_value = if let Some(default_value) = &item.default {
                        default_value.as_bool().unwrap_or(false)
                    } else {
                        false
                    };
                    let (key, value) = get_question_value(
                        help_rendered,
                        key,
                        item,
                        serde_json::Value::Bool(default_value),
                        QuestionType::YesNo,
                        &parsed,
                    )?;

                    answers.insert(key, value);
                }
            };
        }

        let context = serde_json::Value::Object(answers);

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
