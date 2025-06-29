use crate::{
    config::{Config, QuestionRendered},
    error::{Error, Result},
    hooks::{confirm_hook_execution, get_hook_files, run_hook},
    ignore::parse_bakerignore_file,
    import::add_templates_in_renderer,
    ioutils::{
        copy_file, create_dir_all, get_output_dir, parse_string_to_json, read_from,
        write_file,
    },
    loader::get_template,
    prompt::{ask_question, confirm},
    renderer::TemplateRenderer,
    template::{
        get_template_engine, operation::TemplateOperation, processor::TemplateProcessor,
    },
    validation::{validate_answer, ValidationError},
};
use clap::{error::ErrorKind, CommandFactory, Parser, ValueEnum};
use serde_json::json;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Clone, ValueEnum, Copy, PartialEq)]
#[value(rename_all = "lowercase")]
pub enum SkipConfirm {
    /// Skip all confirmation prompts
    All,
    /// Skip confirmation when overwriting existing files
    Overwrite,
    /// Skip confirmation when executing pre/post hooks
    Hooks,
}

/// Command-line arguments structure for Baker.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the template directory or git repository URL
    #[arg(value_name = "TEMPLATE")]
    pub template: String,

    /// Directory where the generated project will be created
    #[arg(value_name = "OUTPUT_DIR")]
    pub output_dir: PathBuf,

    /// Force overwrite of existing output directory
    #[arg(short, long)]
    pub force: bool,

    /// Enable verbose logging output
    #[arg(short, long)]
    pub verbose: bool,

    /// Specifies answers to use during template processing.
    ///
    /// Accepts either a JSON string or "-" to read from stdin.
    ///
    /// Format
    ///
    /// The input should be a JSON object with key-value pairs where:
    ///
    /// - keys are variable names from the template
    ///
    /// - values are the corresponding answers
    ///
    /// Arguments
    ///
    /// * If a string is provided, it should contain valid JSON
    ///
    /// * If "-" is provided, JSON will be read from stdin
    ///
    /// * If None, no predefined answers will be used
    ///
    /// Example
    ///
    /// Provide answers directly
    ///
    /// > baker template_dir output_dir --answers='{"name": "John", "age": 30}'
    ///
    /// Read answers from stdin
    ///
    /// > echo '{"name": "John"}' | baker template_dir output_dir --answers=-
    ///
    #[arg(short, long)]
    pub answers: Option<String>,

    /// Controls which confirmation prompts should be skipped during template processing.
    /// Multiple flags can be combined to skip different types of confirmations.
    ///
    /// Examples
    ///
    /// Skip all confirmation prompts
    ///
    /// > baker --skip-confirms=all
    ///
    /// Skip only file overwrite confirmations
    ///
    /// > baker --skip-confirms=overwrite
    ///
    /// Skip both overwrite and hooks confirmations
    ///
    /// > baker --skip-confirms=overwrite,hooks
    ///
    #[arg(long = "skip-confirms", value_delimiter = ',')]
    #[arg(value_enum)]
    pub skip_confirms: Vec<SkipConfirm>,

    /// Skip interactive prompts if answers are already provided
    /// Use with --answers to create a fully non-interactive workflow
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,
}

/// Parses command line arguments and returns the Args structure.
///
/// # Returns
/// * `Args` - Parsed command line arguments
///
/// # Exits
/// * With status code 1 if required arguments are missing
/// * With clap's default error handling for other argument errors
pub fn get_args() -> Args {
    match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            if e.kind() == ErrorKind::MissingRequiredArgument {
                Args::command()
                    .help_template(
                        r#"{about-section}
{usage-heading} {usage}

{all-args}
{after-help}
"#,
                    )
                    .print_help()
                    .unwrap();
                std::process::exit(1);
            } else {
                e.exit();
            }
        }
    }
}

pub fn run(args: Args) -> Result<()> {
    let mut engine = get_template_engine();

    let output_root = get_output_dir(args.output_dir, args.force)?;

    let template_root = get_template(
        args.template.as_str(),
        args.skip_confirms.contains(&crate::cli::SkipConfirm::All)
            || args.skip_confirms.contains(&crate::cli::SkipConfirm::Overwrite),
    )?;

    let config = Config::load_config(&template_root)?;

    let Config::V1(config) = config;
    config.validate()?;

    add_templates_in_renderer(&template_root, &config, &mut engine);

    let pre_hook_filename = engine.render(
        &config.pre_hook_filename,
        &json!({}),
        Some(&config.pre_hook_filename),
    )?;
    let post_hook_filename = engine.render(
        &config.post_hook_filename,
        &json!({}),
        Some(&config.post_hook_filename),
    )?;

    let execute_hooks = confirm_hook_execution(
        &template_root,
        args.skip_confirms.contains(&crate::cli::SkipConfirm::All)
            || args.skip_confirms.contains(&crate::cli::SkipConfirm::Hooks),
        &pre_hook_filename,
        &post_hook_filename,
    )?;

    let (pre_hook_file, post_hook_file) =
        get_hook_files(&template_root, &pre_hook_filename, &post_hook_filename);

    // Execute pre-generation hook
    let pre_hook_stdout = if execute_hooks && pre_hook_file.exists() {
        log::debug!("Executing pre-hook: {}", pre_hook_file.display());
        run_hook(&template_root, &output_root, &pre_hook_file, None)?
    } else {
        None
    };

    // Retrieve answers from pre-hook and command line and merge them
    let mut answers = serde_json::Map::new();

    if let Some(result) = pre_hook_stdout {
        log::debug!(
            "Pre-hook stdout content (attempting to parse as JSON answers): {result}"
        );

        let pre_answers = serde_json::from_str::<serde_json::Value>(&result).map_or_else(
            |e| {
                log::warn!("Failed to parse hook output as JSON: {e}");
                serde_json::Map::new()
            },
            |value| match value {
                serde_json::Value::Object(map) => map,
                _ => serde_json::Map::new(),
            },
        );
        answers.extend(pre_answers);
    }

    if let Some(answers_arg) = args.answers {
        // From command line argument
        let answers_str =
            if answers_arg == "-" { read_from(std::io::stdin())? } else { answers_arg };
        let cli_answers = parse_string_to_json(answers_str)?;
        answers.extend(cli_answers);
    }

    for (key, question) in config.questions {
        loop {
            let QuestionRendered { help, default, ask_if, .. } =
                question.render(&key, &json!(answers), &engine);

            // Determine if we should skip interactive prompting based on:
            // 1. User explicitly requested non-interactive mode with --non-interactive flag, OR
            // 2. The template's ask_if condition evaluated to false for this question
            let skip_user_prompt = args.non_interactive || !ask_if;

            if skip_user_prompt {
                // Skip to the next question if an answer for this key is already provided
                if answers.contains_key(&key) {
                    break;
                }

                // Use the template's default value if one was specified
                if !question.default.is_null() {
                    answers.insert(key.clone(), default.clone());
                    break;
                }
            }

            let answer = match ask_question(&question, &default, help) {
                Ok(answer) => answer,
                Err(err) => match err {
                    Error::JSONParseError(_) | Error::YAMLParseError(_) => {
                        println!("{err}");
                        continue;
                    }
                    _ => return Err(err),
                },
            };

            answers.insert(key.clone(), answer.clone());
            let _answers = serde_json::Value::Object(answers.clone());

            match validate_answer(&question, &answer, &engine, &_answers) {
                Ok(_) => break,
                Err(err) => match err {
                    ValidationError::JsonSchema(msg) => println!("{msg}"),
                    ValidationError::FieldValidation(msg) => println!("{msg}"),
                },
            }
        }
    }

    let answers = serde_json::Value::Object(answers);

    // Process ignore patterns
    let bakerignore = parse_bakerignore_file(&template_root)?;

    let processor = TemplateProcessor::new(
        &engine,
        &template_root,
        &output_root,
        &answers,
        &bakerignore,
        config.template_suffix.as_str(),
    );

    // Process template files
    for dir_entry in WalkDir::new(&template_root) {
        let template_entry = dir_entry?.path().to_path_buf();
        match processor.process(&template_entry) {
            Ok(file_operation) => {
                let user_confirmed_overwrite = match &file_operation {
                    TemplateOperation::Write { target, target_exists, .. }
                    | TemplateOperation::Copy { target, target_exists, .. } => {
                        let skip_prompt =
                            args.skip_confirms.contains(&crate::cli::SkipConfirm::All)
                                || args
                                    .skip_confirms
                                    .contains(&crate::cli::SkipConfirm::Overwrite)
                                || !target_exists;
                        let user_confirmed = confirm(
                            skip_prompt,
                            format!("Overwrite {}?", target.display()),
                        )?;

                        if user_confirmed {
                            match &file_operation {
                                TemplateOperation::Copy { source, .. } => {
                                    copy_file(&source, &target)?
                                }
                                TemplateOperation::Write { content, .. } => {
                                    write_file(content, target)?
                                }
                                _ => unreachable!(),
                            };
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
                log::info!("{message}");
            }
            Err(e) => match e {
                Error::ProcessError { .. } => log::warn!("{e}"),
                _ => log::error!("{e}"),
            },
        }
    }

    // Execute post-generation hook
    if execute_hooks && post_hook_file.exists() {
        log::debug!("Executing post-hook: {}", post_hook_file.display());
        let post_hook_stdout =
            run_hook(&template_root, &output_root, &post_hook_file, Some(&answers))?;

        if let Some(result) = post_hook_stdout {
            log::debug!("Post-hook stdout content: {result}");
        }
    }

    println!("Template generation completed successfully in {}.", output_root.display());
    Ok(())
}
