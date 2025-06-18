/// Handles argument parsing.
pub mod cli;

mod cli_answers;
/// CLI submodules
mod cli_args;
mod cli_hooks;
mod cli_runner;

/// Defines custom error types.
pub mod error;

/// Pre and post generation hook processing.
pub mod hooks;

/// Processes .bakerignore files to exclude specific paths.
pub mod ignore;

/// Template parsing and rendering functionality.
pub mod renderer;

/// User input and interaction handling.
pub mod dialoguer;

/// An abstraction that allows implementing a source for Baker templates.
pub mod loader;

/// A set of helpers for working with the file system.
pub mod ioutils;

/// Core template processing orchestration.
pub mod template;

/// Configuration handling for Baker templates.
pub mod config;

/// Answer validators
pub mod validation;
