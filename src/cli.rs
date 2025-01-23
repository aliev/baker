use crate::{
    config::{Config, IntoQuestionType, QuestionRendered, QuestionType},
    dialoguer::{
        confirm, prompt_boolean, prompt_multiple_choice, prompt_single_choice,
        prompt_text,
    },
    error::{Error, Result},
    hooks::{confirm_hook_execution, get_hook_files, run_hook},
    ignore::parse_bakerignore_file,
    ioutils::{
        copy_file, create_dir_all, get_output_dir, parse_string_to_json, read_from,
        write_file,
    },
    loader::TemplateSource,
    renderer::{MiniJinjaRenderer, TemplateRenderer},
    template::{operation::TemplateOperation, processor::TemplateProcessor},
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
    let engine: Box<dyn TemplateRenderer> = Box::new(MiniJinjaRenderer::new());

    let output_root = get_output_dir(args.output_dir, args.force)?;

    let template_root = TemplateSource::from_string(
        args.template.as_str(),
        args.skip_confirms.contains(&crate::cli::SkipConfirm::All)
            || args.skip_confirms.contains(&crate::cli::SkipConfirm::Overwrite),
    )?;

    let config = Config::load_config(&template_root)?;

    let Config::V1(config) = config;

    let execute_hooks = confirm_hook_execution(
        &template_root,
        args.skip_confirms.contains(&crate::cli::SkipConfirm::All)
            || args.skip_confirms.contains(&crate::cli::SkipConfirm::Hooks),
    )?;

    let (pre_hook_file, post_hook_file) = get_hook_files(&template_root);

    // Execute pre-generation hook
    let pre_hook_stdout = if execute_hooks && pre_hook_file.exists() {
        run_hook(&template_root, &output_root, &pre_hook_file, None, None, true)?
    } else {
        None
    };

    // Retrieves answers from the `--answers`, stdin, or `pre_hook`
    let buf = if let Some(answers) = args.answers {
        Some(if answers == "-" { read_from(std::io::stdin())? } else { answers })
    } else if let Some(pre_hook_stdout) = pre_hook_stdout {
        Some(read_from(pre_hook_stdout)?)
    } else {
        None
    };

    // Parses retrieved answers to JSON or returns the default map
    let mut answers = if let Some(buf) = buf {
        parse_string_to_json(buf)?
    } else {
        serde_json::Map::new()
    };

    for (key, question) in config.questions {
        let QuestionRendered { help, default, ask_if, .. } =
            question.render(&key, &json!(answers), engine.as_ref());

        let answer = if ask_if {
            // Asks questions
            match question.into_question_type() {
                QuestionType::MultipleChoice => {
                    prompt_multiple_choice(question.choices, default, help)?
                }
                QuestionType::Boolean => prompt_boolean(default, help)?,
                QuestionType::SingleChoice => {
                    prompt_single_choice(question.choices, default, help)?
                }
                QuestionType::Text => prompt_text(&question, default, help)?,
            }
        } else {
            default
        };
        answers.insert(key, answer);
    }

    let answers = serde_json::Value::Object(answers);

    // Process ignore patterns
    let bakerignore = parse_bakerignore_file(&template_root)?;

    let processor = TemplateProcessor::new(
        engine.as_ref(),
        &template_root,
        &output_root,
        &answers,
        &bakerignore,
    );

    let mut operations = vec![];

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
                let message = &file_operation.get_message(user_confirmed_overwrite);
                log::info!("{}", message);
                operations.push(file_operation);
            }
            Err(e) => match e {
                Error::ProcessError { .. } => log::warn!("{}", e),
                _ => log::error!("{}", e),
            },
        }
    }

    // Execute post-generation hook
    if execute_hooks && post_hook_file.exists() {
        run_hook(
            &template_root,
            &output_root,
            &post_hook_file,
            Some(&answers),
            Some(operations),
            false,
        )?;
    }

    println!("Template generation completed successfully in {}.", output_root.display());
    Ok(())
}
