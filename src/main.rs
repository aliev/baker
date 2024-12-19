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
    processor::{FileOperation, Processor},
    prompt::DialoguerPrompter,
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

fn write_file<P: AsRef<Path>>(content: &str, dest_path: P) -> Result<()> {
    let dest_path = dest_path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_path = if dest_path.is_absolute() {
        dest_path.to_path_buf()
    } else {
        base_path.join(dest_path)
    };

    if let Some(parent) = abs_path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::IoError)?;
    }
    std::fs::write(abs_path, content).map_err(Error::IoError)
}

fn copy_file<P: AsRef<Path>>(source_path: P, dest_path: P) -> Result<()> {
    let dest_path = dest_path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_dest = if dest_path.is_absolute() {
        dest_path.to_path_buf()
    } else {
        base_path.join(dest_path)
    };

    if let Some(parent) = abs_dest.parent() {
        std::fs::create_dir_all(parent).map_err(Error::IoError)?;
    }
    std::fs::copy(source_path, abs_dest).map(|_| ()).map_err(Error::IoError)
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
    let engine = Box::new(MiniJinjaRenderer::new());
    let prompt = Box::new(DialoguerPrompter::new());

    let output_root = get_output_dir(args.output_dir, args.force)?;
    let template_root =
        load_template(&*prompt, args.template, args.skip_overwrite_check)?;
    let config = get_config(&template_root)?;

    let execute_hooks =
        confirm_hook_execution(&*prompt, &template_root, args.skip_hooks_check)?;

    let (pre_hook_file, post_hook_file) = get_hook_files(&template_root);

    // Execute pre-generation hook
    let pre_hook_stdout = if execute_hooks && pre_hook_file.exists() {
        run_hook(&template_root, &output_root, &pre_hook_file, None, true)?
    } else {
        None
    };

    let preloaded_answers = get_answers_from(args.stdin, pre_hook_stdout)?;
    let answers = get_answers(&*engine, &*prompt, config.questions, preloaded_answers)?;

    // Process ignore patterns
    let ignored_patterns = parse_bakerignore_file(&template_root)?;

    let processor = Processor::new(
        &*engine,
        &*prompt,
        &template_root,
        &output_root,
        args.skip_overwrite_check,
        &answers,
        &ignored_patterns,
    );

    // Process template files
    for dir_entry in WalkDir::new(&template_root) {
        let raw_entry = dir_entry.map_err(|e| Error::TemplateError(e.to_string()))?;
        let template_entry = raw_entry.path().to_path_buf();
        match processor.process(&template_entry) {
            Ok(result) => {
                if let Some(operation) = result.operation {
                    let target = match operation {
                        FileOperation::Copy { target } => {
                            copy_file(&result.source, &target)?;
                            target
                        }
                        FileOperation::Write { target, content } => {
                            write_file(&content, &target)?;
                            target
                        }
                    };
                    println!("{}: '{}'", result.action, target.display());
                }
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
