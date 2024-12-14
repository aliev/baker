//! Error handling for the Baker application.
//! Defines custom error types and results used throughout the application.

use std::process::ExitStatus;
use thiserror::Error;

/// Custom error types for Baker operations.
///
/// This enum represents all possible errors that can occur within the Baker application.
/// It implements the standard Error trait through thiserror's derive macro.
#[derive(Error, Debug)]
pub enum Error {
    /// Represents errors that occur during file system operations
    #[error("IO error: {0}.")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse config file. Original error: {0}.")]
    ConfigParseError(#[from] serde_yaml::Error),

    #[error("Failed to parse .bakerignore file. Original error: {0}")]
    GlobSetParseError(#[from] globset::Error),

    #[error("Failed to display confirmation prompt. Original error: {0}")]
    PromptError(#[from] dialoguer::Error),

    #[error("Failed to clone repository. Original error: {0}")]
    Git2Error(#[from] git2::Error),

    #[error("Failed to render. Original error: {0}")]
    MinijinjaError(#[from] minijinja::Error),

    /// Represents errors that occur during template processing
    #[error("Template error: {0}.")]
    TemplateError(String),

    /// Represents errors that occur during configuration parsing or processing
    #[error("No configuration file found in '{template_dir}'. Tried: {config_files}.")]
    ConfigError { template_dir: String, config_files: String },

    /// When the Hook has executed but finished with an error.
    #[error("Hook execution failed with status: {status}")]
    HookExecutionError { status: ExitStatus },

    /// Represents validation failures in user input or data
    #[error("Validation error: {0}.")]
    ValidationError(String),

    /// Represents errors in processing .bakerignore files
    #[error("BakerIgnore error: {0}.")]
    BakerIgnoreError(String),

    #[error("Cannot proceed: output directory '{output_dir}' already exists. Use --force to overwrite it.")]
    OutputDirectoryExistsError { output_dir: String },
    #[error("Cannot proceed: template directory '{template_dir}' does not exist.")]
    TemplateDoesNotExistsError { template_dir: String },
    #[error("Cannot proceed: invalid type of template source.")]
    TemplateSourceInvalidError,

    #[error("Cannot process the source path: '{source_path}'. Original error: {e}")]
    ProcessError { source_path: String, e: String },
}

/// Convenience type alias for Results with BakerError as the error type.
///
/// # Type Parameters
/// * `T` - The type of the success value
pub type Result<T> = std::result::Result<T, Error>;

/// Default error handler that prints the error and exits the program.
///
/// # Arguments
/// * `err` - The BakerError to handle
///
/// # Behavior
/// Prints the error message to stderr and exits with status code 1
pub fn default_error_handler(err: Error) {
    eprintln!("{}", err);
    std::process::exit(1);
}
