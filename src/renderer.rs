//! Template renderer and rendering functionality for Baker.
//! Handles both local filesystem and git repository templates with support
//! for MiniJinja template processing.
use crate::error::{Error, Result};
use minijinja::Environment;

/// Trait for template rendering engines.
pub trait TemplateRenderer {
    /// Renders a template string with the given context.
    ///
    /// # Arguments
    /// * `template` - Template string to render
    /// * `context` - Context variables for rendering
    ///
    /// # Returns
    /// * `BakerResult<String>` - Rendered template string
    fn render(&self, template: &str, context: &serde_json::Value) -> Result<String>;
}

/// MiniJinja-based template rendering engine.
pub struct MiniJinjaRenderer {
    /// MiniJinja environment instance
    env: Environment<'static>,
}

impl MiniJinjaRenderer {
    /// Creates a new MiniJinjaEngine instance with default environment.
    pub fn new() -> Self {
        let env = Environment::new();
        Self { env }
    }
}

impl Default for MiniJinjaRenderer {
    fn default() -> Self {
        MiniJinjaRenderer::new()
    }
}

impl TemplateRenderer for MiniJinjaRenderer {
    /// Renders a template string using MiniJinja.
    ///
    /// # Arguments
    /// * `template` - Template string to render
    /// * `context` - JSON context for variable interpolation
    ///
    /// # Returns
    /// * `BakerResult<String>` - Rendered template string
    ///
    /// # Errors
    /// * `BakerError::TemplateError` if:
    ///   - Template addition fails
    ///   - Template retrieval fails
    ///   - Template rendering fails
    fn render(&self, template: &str, context: &serde_json::Value) -> Result<String> {
        let mut env = self.env.clone();
        env.add_template("temp", template).map_err(Error::MinijinjaError)?;

        let tmpl = env.get_template("temp").map_err(Error::MinijinjaError)?;

        tmpl.render(context).map_err(Error::MinijinjaError)
    }
}
