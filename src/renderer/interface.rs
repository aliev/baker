use crate::error::Result;
use std::path::Path;

/// Trait for template rendering engines.
pub trait TemplateRenderer {
    /// Adds a template to the renderer's template collection.
    ///
    /// # Arguments
    /// * `name` - Name to identify the template
    /// * `template` - Template content as string
    ///
    /// # Returns
    /// * `Result<(), minijinja::Error>` - Success or MiniJinja error
    fn add_template(
        &mut self,
        name: &str,
        template: &str,
    ) -> Result<(), minijinja::Error>;

    /// Renders a template string with the given context.
    ///
    /// # Arguments
    /// * `template` - Template string to render
    /// * `context` - Context variables for rendering
    /// * `template_name` - Optional name for the template (used in error messages)
    ///
    /// # Returns
    /// * `Result<String>` - Rendered template string
    fn render(
        &self,
        template: &str,
        context: &serde_json::Value,
        template_name: Option<&str>,
    ) -> Result<String>;

    /// Renders a path with the given context.
    ///
    /// # Arguments
    /// * `template_path` - Path to render
    /// * `context` - Context variables for rendering
    ///
    /// # Returns
    /// * `Result<String>` - Rendered path as string
    fn render_path(
        &self,
        template_path: &Path,
        context: &serde_json::Value,
    ) -> Result<String>;

    /// Executes a template expression and returns whether it evaluates to true.
    ///
    /// # Arguments
    /// * `expr` - Expression to evaluate
    /// * `context` - Context variables for evaluation
    ///
    /// # Returns
    /// * `Result<bool>` - Whether the expression evaluates to true
    fn execute_expression(&self, expr: &str, context: &serde_json::Value)
        -> Result<bool>;
}
