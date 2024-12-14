//! Baker's main application entry point and orchestration logic.
//! Handles command-line argument parsing, template processing flow,
//! and coordinates interactions between different modules.

use baker::{
    cli::{get_args, Args},
    config::get_config,
    error::{default_error_handler, Error, Result},
    hooks::{get_hooks_dirs, get_path_if_exists, run_hook},
    ignore::{parse_bakerignore_file, IGNORE_FILE},
    parser::{
        get_answers, get_answers_from, load_from_hook, load_from_stdin, AnswerSource,
    },
    processor::{ensure_output_dir, process_directory},
    prompt::prompt_confirm_hooks_execution,
    template::{get_template_dir, MiniJinjaEngine, TemplateEngine},
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
    let output_dir = ensure_output_dir(args.output_dir, args.force)?;

    // TEMPLATE PART
    let template_dir = get_template_dir(args.template)?;

    // Load and parse configuration
    let config = get_config(&template_dir)?;

    // HOOKS PART
    let (pre_hook_dir, post_hook_dir) = get_hooks_dirs(&template_dir);

    let execute_hooks = if pre_hook_dir.exists() || post_hook_dir.exists() {
        prompt_confirm_hooks_execution(
                args.skip_hooks_check,
                format!(
                    "WARNING: This template contains the following hooks that will execute commands on your system:\n{}{}{}",
                    get_path_if_exists(&post_hook_dir),
                    get_path_if_exists(&pre_hook_dir),
                    "Do you want to run these hooks?",
                ),
            )?
    } else {
        false
    };

    // Execute pre-generation hook
    let pre_hook_stdout = if execute_hooks && pre_hook_dir.exists() {
        run_hook(&template_dir, &output_dir, &pre_hook_dir, None, true)?
    } else {
        None
    };

    // HOOKS PART END

    let answers_source = get_answers_from(args.stdin, pre_hook_stdout)?;

    let preloaded_answers = match answers_source {
        AnswerSource::Stdin => load_from_stdin(),
        AnswerSource::PreHookStdout(stdout) => load_from_hook(stdout),
        AnswerSource::None => Ok(serde_json::Value::Null),
    }?;

    let engine: Box<dyn TemplateEngine> = Box::new(MiniJinjaEngine::new());
    let answers = get_answers(&*engine, config.questions, preloaded_answers)?;

    // Process ignore patterns
    let ignored_set = parse_bakerignore_file(template_dir.join(IGNORE_FILE))?;

    // Process template files
    for entry in WalkDir::new(&template_dir) {
        let entry = entry.map_err(|e| Error::TemplateError(e.to_string()))?;
        let path = entry.path();
        if let Err(e) = process_directory(
            path,
            &template_dir,
            &output_dir,
            &answers,
            &*engine,
            &ignored_set,
            args.overwrite,
        ) {
            match e {
                Error::ProcessError { .. } => {
                    log::warn!("{}", e)
                }
                _ => log::error!("{}", e),
            }
        }
    }

    // Execute post-generation hook
    if execute_hooks && post_hook_dir.exists() {
        run_hook(&template_dir, &output_dir, &post_hook_dir, Some(&answers), false)?;
    }

    println!("Template generation completed successfully in {}.", output_dir.display());
    Ok(())
}
