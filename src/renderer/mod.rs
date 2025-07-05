//! Template rendering engine for Baker
//!
//! This module provides template rendering capabilities using MiniJinja.
//! It includes various built-in filters for string manipulation and formatting.
//!
//! The module is structured as:
//! - `interface`: Core trait definitions for template rendering
//! - `minijinja`: MiniJinja-based implementation of the template renderer
//! - `filters`: Custom filters for template processing

pub mod filters;
pub mod interface;
pub mod minijinja;

// Re-export the main types and traits for convenience
pub use interface::TemplateRenderer;
pub use minijinja::MiniJinjaRenderer;

/// Convenience function to create the default template renderer
pub fn new_renderer() -> impl TemplateRenderer {
    MiniJinjaRenderer::new()
}
