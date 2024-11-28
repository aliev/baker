//! Baker is a template processing system for project scaffolding.
//! It provides functionality to create projects from templates with customizable hooks,
//! configurations, and template processing capabilities.

/// Command-line interface module for the Baker application.
/// Handles argument parsing, help text formatting, and command execution.
pub mod cli;

/// Configuration handling for Baker templates.
/// Supports JSON and YAML formats (baker.json, baker.yml, baker.yaml).
/// Handles variable interpolation and validation.
pub mod config;

/// Error types and handling for the Baker application.
/// Defines custom error types and results used throughout the application.
pub mod error;

/// Pre and post generation hook processing.
/// Handles execution of scripts in:
/// - hooks/pre_gen_project - Run before template generation
/// - hooks/post_gen_project - Run after template generation
/// Provides safety checks and context passing.
pub mod hooks;

/// File and directory ignore patterns.
/// Processes .bakerignore files to exclude specific paths.
/// Similar to .gitignore functionality but specific to Baker.
pub mod ignore;

/// Core template processing orchestration.
/// Combines all components to generate the final output:
/// - Template loading
/// - Variable interpolation
/// - File/directory creation
/// - Hook execution
pub mod processor;

/// Template parsing and rendering functionality.
/// Handles the actual template processing logic:
/// - Local and Git template sources
/// - MiniJinja template rendering
/// - Variable interpolation
pub mod template;
