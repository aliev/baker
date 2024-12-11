//! Baker's main application entry point and orchestration logic.
//! Handles command-line argument parsing, template processing flow,
//! and coordinates interactions between different modules.

use std::io::Read;

use baker::{
    cli::{get_args, Args},
    config::{load_config, Config, CONFIG_FILES},
    error::{default_error_handler, BakerError, BakerResult},
    hooks::{get_hooks_dirs, get_path_if_exists, run_hook},
    ignore::{parse_bakerignore_file, IGNORE_FILE},
    parser::get_answers,
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
    let source = if let Some(source) = TemplateSource::from_string(&args.template) {
        source
    } else {
        return Err(BakerError::TemplateError(format!(
            "invalid template source: {}",
            args.template
        )));
    };

    let output_dir = ensure_output_dir(args.output_dir, args.force)?;
    let loader: Box<dyn TemplateLoader> = match source {
        TemplateSource::Git(_) => Box::new(GitLoader::new()),
        TemplateSource::FileSystem(_) => Box::new(LocalLoader::new()),
    };
    let template_dir = loader.load(&source)?;
    // Load and parse configuration
    let config_content = load_config(&template_dir, &CONFIG_FILES)?;

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

    // Template processor initialization
    let engine: Box<dyn TemplateEngine> = Box::new(MiniJinjaEngine::new());
    // TODO: map_err
    let config: Config = serde_yaml::from_str(&config_content).unwrap();

    // Execute pre-generation hook
    let output = if execute_hooks && pre_hook_dir.exists() {
        run_hook(&template_dir, &output_dir, &pre_hook_dir, None, true)?
    } else {
        None
    };

    let default_answers = if !args.answers.is_empty() {
        serde_json::from_str(&args.answers).unwrap_or_else(|_| serde_json::Value::Null)
    } else if let Some(mut stdout) = output {
        let mut buf = String::new();
        stdout.read_to_string(&mut buf).expect("Failed to read stdout");
        serde_json::from_str(&buf).unwrap_or_else(|_| serde_json::Value::Null)
    } else {
        serde_json::Value::Null
    };

    dbg!(&default_answers);

    let answers = get_answers(&*engine, config, default_answers)?;

    // Process ignore patterns
    let ignored_set = parse_bakerignore_file(template_dir.join(IGNORE_FILE))?;

    // Process template files
    for entry in WalkDir::new(&template_dir) {
        if let Err(e) = process_entry(
            entry,
            &template_dir,
            &output_dir,
            &answers,
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
    if execute_hooks && post_hook_dir.exists() {
        run_hook(&template_dir, &output_dir, &post_hook_dir, Some(&answers), false)?;
    }

    println!("Template generation completed successfully in {}.", output_dir.display());
    Ok(())
}
