//! Baker is a template processing system for project scaffolding.
//! It provides functionality to create projects from templates with customizable hooks,
//! configurations, and template processing capabilities.

/// Command-line interface module for the Baker application
pub mod cli;

/// Configuration handling for Baker templates
/// Supports JSON and YAML formats (baker.json, baker.yml, baker.yaml)
pub mod config;

/// Error types and handling for the Baker application
pub mod error;

/// Pre and post generation hook processing
/// Handles execution of scripts in:
/// - hooks/pre_gen_project
/// - hooks/post_gen_project
pub mod hooks;

/// File and directory ignore patterns
/// Processes .bakerignore files to exclude specific paths
pub mod ignore;

/// Core template processing orchestration
/// Combines all components to generate the final output
pub mod processor;

/// User input and interaction handling
pub mod prompt;

/// Template parsing and rendering functionality
/// Handles the actual template processing logic
pub mod template;
