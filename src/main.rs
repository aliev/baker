//! Baker's main application entry point and orchestration logic.
//! Handles command-line argument parsing, template processing flow,
//! and coordinates interactions between different modules.

use std::path::{Path, PathBuf};

use baker::{
    cli::{get_args, Args},
    config::get_config,
    error::{default_error_handler, Error, Result},
    hooks::{confirm_hook_execution, get_hook_files, run_hook},
    ignore::parse_bakerignore_file,
    loader::load_template,
    parser::{get_answers, get_answers_from},
    processor::process_template_entry,
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

/// Ensures the output directory exists and is safe to write to.
///
/// # Arguments
/// * `output_dir` - Target directory path for generated output
/// * `force` - Whether to overwrite existing directory
///
/// # Returns
/// * `BakerResult<PathBuf>` - Validated output directory path
///
/// # Errors
/// * Returns `BakerError::ConfigError` if directory exists and force is false
pub fn get_output_dir<P: AsRef<Path>>(output_dir: P, force: bool) -> Result<PathBuf> {
    let output_dir = output_dir.as_ref();
    if output_dir.exists() && !force {
        return Err(Error::OutputDirectoryExistsError {
            output_dir: output_dir.display().to_string(),
        });
    }
    Ok(output_dir.to_path_buf())
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
    let output_root = get_output_dir(args.output_dir, args.force)?;
    let template_root = load_template(args.template, args.skip_overwrite_check)?;

    let config = get_config(&template_root)?;

    let execute_hooks = confirm_hook_execution(&template_root, args.skip_hooks_check)?;

    let (pre_hook_file, post_hook_file) = get_hook_files(&template_root);

    // Execute pre-generation hook
    let pre_hook_stdout = if execute_hooks && pre_hook_file.exists() {
        run_hook(&template_root, &output_root, &pre_hook_file, None, true)?
    } else {
        None
    };

    let preloaded_answers = get_answers_from(args.stdin, pre_hook_stdout)?;

    let engine = Box::new(MiniJinjaRenderer::new());
    let answers = get_answers(&*engine, config.questions, preloaded_answers)?;

    // Process ignore patterns
    let ignored_patterns = parse_bakerignore_file(&template_root)?;

    // Process template files
    for dir_entry in WalkDir::new(&template_root) {
        let raw_entry = dir_entry.map_err(|e| Error::TemplateError(e.to_string()))?;
        let template_entry = raw_entry.path().to_path_buf();

        if let Err(e) = process_template_entry(
            &template_root,
            &output_root,
            &template_entry,
            &answers,
            &*engine,
            &ignored_patterns,
            args.skip_overwrite_check,
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
        run_hook(&template_root, &output_root, &post_hook_file, Some(&answers), false)?;
    }

    println!("Template generation completed successfully in {}.", output_root.display());
    Ok(())
}
