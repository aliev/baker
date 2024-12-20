//! Template renderer and rendering functionality for Baker.
//! Handles both local filesystem and git repository templates with support
//! for MiniJinja template processing.
use crate::error::{Error, Result};
use minijinja::Environment;
use std::path::Path;

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
    fn render_path(
        &self,
        template_path: &Path,
        context: &serde_json::Value,
    ) -> Result<String>;
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

    fn render_internal(
        &self,
        template: &str,
        context: &serde_json::Value,
    ) -> Result<String> {
        let mut env = self.env.clone();
        env.add_template("temp", template).map_err(Error::MinijinjaError)?;
        let tmpl = env.get_template("temp").map_err(Error::MinijinjaError)?;
        tmpl.render(context).map_err(Error::MinijinjaError)
    }
}

impl Default for MiniJinjaRenderer {
    fn default() -> Self {
        MiniJinjaRenderer::new()
    }
}

impl TemplateRenderer for MiniJinjaRenderer {
    fn render(&self, template: &str, context: &serde_json::Value) -> Result<String> {
        self.render_internal(template, context)
    }
    fn render_path(
        &self,
        template_path: &Path,
        context: &serde_json::Value,
    ) -> Result<String> {
        let path_str = template_path.to_str().ok_or_else(|| Error::ProcessError {
            source_path: template_path.display().to_string(),
            e: "Cannot convert source_path to string.".to_string(),
        })?;

        self.render_internal(path_str, context).map_err(|e| Error::ProcessError {
            source_path: path_str.to_string(),
            e: e.to_string(),
        })
    }
}
