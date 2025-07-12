//! # Baker - Project Scaffolding Tool
//!
//! Baker is a powerful template-based project generator that helps you create
//! new projects from predefined templates with dynamic content generation.
//!
//! ## Core Components
//!
//! - **CLI**: Command-line interface and argument parsing
//! - **Configuration**: Template configuration management and validation
//! - **Templates**: Template processing and rendering engine
//! - **Prompts**: Interactive user input handling
//! - **Loaders**: Template source management (local/git)
//!
//! ## Quick Start
//!
//! ```ignore
//! use baker::cli::{get_args, run};
//!
//! let args = get_args();
//! run(args)?;
//! ```

/// Handles argument parsing.
pub mod cli;

/// Application-wide constants.
pub mod constants;

/// Defines custom error types.
pub mod error;

/// Processes .bakerignore files to exclude specific paths.
pub mod ignore;

/// Template parsing and rendering functionality.
pub mod renderer;

/// User input and interaction handling.
pub mod prompt;

/// An abstraction that allows implementing a source for Baker templates.
pub mod loader;

/// Core template processing orchestration.
pub mod template;

/// Configuration handling for Baker templates.
pub mod config;

/// Extension traits for built-in Rust types.
pub mod ext;

pub mod metadata;
