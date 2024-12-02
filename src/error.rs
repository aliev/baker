//! Error handling for the Baker application.
//! Defines custom error types and results used throughout the application.

use std::io;
use thiserror::Error;

/// Custom error types for Baker operations.
///
/// This enum represents all possible errors that can occur within the Baker application.
/// It implements the standard Error trait through thiserror's derive macro.
#[derive(Error, Debug)]
pub enum BakerError {
    /// Represents errors that occur during file system operations
    #[error("IO error: {0}.")]
    IoError(#[from] io::Error),

    /// Represents errors that occur during template processing
    #[error("Template error: {0}.")]
    TemplateError(String),

    /// Represents errors that occur during configuration parsing or processing
    #[error("Configuration error: {0}.")]
    ConfigError(String),

    /// Represents errors that occur during hook script execution
    #[error("Hook execution error: {0}.")]
    HookError(String),

    /// Represents validation failures in user input or data
    #[error("Validation error: {0}.")]
    ValidationError(String),

    /// Represents errors in processing .bakerignore files
    #[error("BakerIgnore error: {0}.")]
    BakerIgnoreError(String),
}

/// Convenience type alias for Results with BakerError as the error type.
///
/// # Type Parameters
/// * `T` - The type of the success value
pub type BakerResult<T> = Result<T, BakerError>;

/// Default error handler that prints the error and exits the program.
///
/// # Arguments
/// * `err` - The BakerError to handle
///
/// # Behavior
/// Prints the error message to stderr and exits with status code 1
pub fn default_error_handler(err: BakerError) {
    eprintln!("{}", err);
    std::process::exit(1);
}
