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

    /// Represents errors that occur during template processing
    #[error("Template error: {0}.")]
    TemplateError(String),

    /// Represents errors that occur during configuration parsing or processing
    #[error("No configuration file found in '{template_dir}'. Tried: {config_files}.")]
    ConfigError { template_dir: String, config_files: String },

    /// Represents errors that occur during hook script execution
    #[error("Hook execution error: {0}.")]
    HookError(String),

    #[error("Hook execution failed with status: {status}")]
    HookExecutionError { status: ExitStatus },

    /// Represents validation failures in user input or data
    #[error("Validation error: {0}.")]
    ValidationError(String),

    /// Represents errors in processing .bakerignore files
    #[error("BakerIgnore error: {0}.")]
    BakerIgnoreError(String),

    #[error("Failed to parse .bakerignore file. Original error: {e}")]
    GlobSetParseError { e: globset::Error },

    #[error("Failed to display confirmation prompt. Original error: {e}")]
    PromptError { e: dialoguer::Error },

    #[error("Cannot proceed: output directory '{output_dir}' already exists. Use --force to overwrite it.")]
    OutputDirectoryExistsError { output_dir: String },
}

impl Error {
    pub fn from_dialoguer_error(e: dialoguer::Error) -> Self {
        Error::PromptError { e }
    }

    pub fn from_glob_set_error(e: globset::Error) -> Self {
        Error::GlobSetParseError { e }
    }
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
