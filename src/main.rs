//! Baker's main application entry point and orchestration logic.
//! Handles command-line argument parsing, template processing flow,
//! and coordinates interactions between different modules.

use baker::{
    cli::{get_args, Args},
    config::{Config, IntoQuestionType, QuestionType},
    dialoguer::{
        confirm, prompt_boolean, prompt_multiple_choice, prompt_single_choice,
        prompt_text,
    },
    error::{default_error_handler, Error, Result},
    hooks::{confirm_hook_execution, get_hook_files, run_hook},
    ignore::parse_bakerignore_file,
    ioutils::{copy_file, create_dir_all, get_output_dir, read_from, write_file},
    loader::TemplateSource,
    renderer::{MiniJinjaRenderer, TemplateRenderer},
    template::{operation::TemplateOperation, processor::TemplateProcessor},
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
fn run(args: Args) -> Result<()> {
    let engine: Box<dyn TemplateRenderer> = Box::new(MiniJinjaRenderer::new());

    let output_root = get_output_dir(args.output_dir, args.force)?;

    let template_root =
        TemplateSource::from_string(args.template.as_str(), args.skip_overwrite_check)?;

    let config = Config::new()
        .from_json(&template_root.join("baker.json"))
        .from_yaml(&template_root.join("baker.yaml"))
        .from_yml(&template_root.join("baker.yml"))
        .build()?;

    let execute_hooks = confirm_hook_execution(&template_root, args.skip_hooks_check)?;

    let (pre_hook_file, post_hook_file) = get_hook_files(&template_root);

    // Execute pre-generation hook
    let pre_hook_stdout = if execute_hooks && pre_hook_file.exists() {
        run_hook(&template_root, &output_root, &pre_hook_file, None, true)?
    } else {
        None
    };

    // Gets answers either from stdin or pre_hook stdout.
    // Maybe rename answers to default / defaults
    // to be consistent with config file.
    let mut answers = if args.stdin {
        read_from(std::io::stdin())?
    } else if let Some(pre_hook_stdout) = pre_hook_stdout {
        read_from(pre_hook_stdout)?
    } else {
        serde_json::Map::new()
    };

    for (key, question) in config.questions {
        let current_context = serde_json::Value::Object(answers.clone());

        let rendered_question = question.render(&current_context, &*engine);

        let before_answered_question = answers.get(&key);

        let answer = if let Some(default_answer_value) = before_answered_question {
            // Gets default answer from whatever context
            default_answer_value.clone()
        } else if rendered_question.ask_if {
            // Asks answer
            match question.into_question_type() {
                QuestionType::MultipleChoice => prompt_multiple_choice(
                    question.choices,
                    rendered_question.default,
                    rendered_question.help.unwrap_or_default(),
                )?,
                QuestionType::Boolean => prompt_boolean(
                    rendered_question.default,
                    rendered_question.help.unwrap_or_default(),
                )?,
                QuestionType::SingleChoice => prompt_single_choice(
                    question.choices,
                    rendered_question.default,
                    rendered_question.help.unwrap_or_default(),
                )?,
                QuestionType::Text => prompt_text(
                    &question,
                    rendered_question.default,
                    rendered_question.help.unwrap_or_default(),
                )?,
            }
        } else {
            rendered_question.default
        };
        answers.insert(key, answer);
    }

    let answers = serde_json::Value::Object(answers);

    // Process ignore patterns
    let bakerignore = parse_bakerignore_file(&template_root)?;

    let processor = TemplateProcessor::new(
        &*engine,
        &template_root,
        &output_root,
        &answers,
        &bakerignore,
    );

    // Process template files
    for dir_entry in WalkDir::new(&template_root) {
        let raw_entry = dir_entry.map_err(|e| Error::TemplateError(e.to_string()))?;
        let template_entry = raw_entry.path().to_path_buf();
        match processor.process(&template_entry) {
            Ok(file_operation) => {
                let user_confirmed_overwrite = match &file_operation {
                    TemplateOperation::Copy { source, target, target_exists } => {
                        let skip_prompt = args.skip_overwrite_check || !target_exists;
                        let user_confirmed = confirm(
                            skip_prompt,
                            format!("Overwrite {}?", target.display()),
                        )?;

                        if user_confirmed {
                            copy_file(&source, &target)?;
                        }

                        user_confirmed
                    }
                    TemplateOperation::Write { target, content, target_exists } => {
                        let skip_prompt = args.skip_overwrite_check || !target_exists;
                        let user_confirmed = confirm(
                            skip_prompt,
                            format!("Overwrite {}?", target.display()),
                        )?;

                        if user_confirmed {
                            write_file(&content, &target)?;
                        }
                        user_confirmed
                    }
                    TemplateOperation::CreateDirectory { target, target_exists } => {
                        if !target_exists {
                            create_dir_all(target)?;
                        }
                        true
                    }
                    TemplateOperation::Ignore { .. } => true,
                };

                let message = file_operation.get_message(user_confirmed_overwrite);
                log::info!("{}", message);
            }
            Err(e) => match e {
                Error::ProcessError { .. } => log::warn!("{}", e),
                _ => log::error!("{}", e),
            },
        }
    }

    // Execute post-generation hook
    if execute_hooks && post_hook_file.exists() {
        run_hook(&template_root, &output_root, &post_hook_file, Some(&answers), false)?;
    }

    println!("Template generation completed successfully in {}.", output_root.display());
    Ok(())
}
