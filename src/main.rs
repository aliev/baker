//! Baker's main application entry point and orchestration logic.
//! Handles command-line argument parsing, template processing flow,
//! and coordinates interactions between different modules.

use baker::{
    cli::{get_args, Args},
    config::get_config,
    error::{default_error_handler, Error, Result},
    hooks::{confirm_hook_execution, get_hook_files, run_hook},
    ignore::{parse_bakerignore_file, IGNORE_FILE},
    loader::load_template,
    parser::{get_answers, get_answers_from},
    processor::{get_output_dir, process_template_entry},
    renderer::MiniJinjaRenderer,
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
    let output_dir = get_output_dir(args.output_dir, args.force)?;
    let template_dir = load_template(args.template)?;

    let config = get_config(&template_dir)?;

    let execute_hooks = confirm_hook_execution(&template_dir, args.skip_hooks_check)?;

    let (pre_hook_file, post_hook_file) = get_hook_files(&template_dir);

    // Execute pre-generation hook
    let pre_hook_stdout = if execute_hooks && pre_hook_file.exists() {
        run_hook(&template_dir, &output_dir, &pre_hook_file, None, true)?
    } else {
        None
    };

    let preloaded_answers = get_answers_from(args.stdin, pre_hook_stdout)?;

    let engine = Box::new(MiniJinjaRenderer::new());
    let answers = get_answers(&*engine, config.questions, preloaded_answers)?;

    // Process ignore patterns
    let ignored_set = parse_bakerignore_file(template_dir.join(IGNORE_FILE))?;

    // Process template files
    for dir_entry in WalkDir::new(&template_dir) {
        let raw_entry = dir_entry.map_err(|e| Error::TemplateError(e.to_string()))?;
        let template_entry = raw_entry.path();
        dbg!(template_entry);
        if let Err(e) = process_template_entry(
            template_entry,
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
    if execute_hooks && post_hook_file.exists() {
        run_hook(&template_dir, &output_dir, &post_hook_file, Some(&answers), false)?;
    }

    println!("Template generation completed successfully in {}.", output_dir.display());
    Ok(())
}
