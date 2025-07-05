//! Template processing engine for Baker
//!
//! This module contains the core template processing components:
//! - `operation`: Defines operations to be performed on templates
//! - `processor`: Contains the logic for processing template files and directories

use crate::renderer::{new_renderer, TemplateRenderer};

pub mod operation;
pub mod processor;

/// Convenience function to create the default template engine
pub fn get_template_engine() -> impl TemplateRenderer {
    new_renderer()
}
