use crate::{error::Result, ioutils::path_to_str};
pub use cruet::{
    case::{
        camel::to_camel_case, kebab::to_kebab_case, pascal::to_pascal_case,
        screaming_snake::to_screaming_snake_case, snake::to_snake_case,
        table::to_table_case, train::to_train_case,
    },
    string::{pluralize::to_plural, singularize::to_singular},
    suffix::foreign_key::to_foreign_key,
};
use log::warn;
use minijinja::Environment;
use regex::Regex;
use serde_json::json;
use std::path::Path;

/// Trait for template rendering engines.
pub trait TemplateRenderer {
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
    ///
    /// # Returns
    /// * `Result<String>` - Rendered template string
    fn render(&self, template: &str, context: &serde_json::Value) -> Result<String>;

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

/// MiniJinja-based template rendering engine.
pub struct MiniJinjaRenderer {
    /// MiniJinja environment instance
    env: Environment<'static>,
    /// Default context that will be merged with any provided context
    default_context: serde_json::Value,
}

fn regex_filter(val: &str, re: &str) -> bool {
    match Regex::new(re) {
        Ok(re) => re.is_match(val),
        Err(err) => {
            warn!("Invalid regex '{}': {}", re, err);
            false
        }
    }
}

impl MiniJinjaRenderer {
    /// Creates a new MiniJinjaEngine instance with default environment.
    pub fn new() -> Self {
        let mut env = Environment::new();
        let default_context = json!({
            "platform": {
                "os": std::env::consts::OS,
                "family": std::env::consts::FAMILY,
                "arch": std::env::consts::ARCH,
            }
        });

        env.add_filter("camel_case", to_camel_case);
        env.add_filter("kebab_case", to_kebab_case);
        env.add_filter("pascal_case", to_pascal_case);
        env.add_filter("screaming_snake_case", to_screaming_snake_case);
        env.add_filter("snake_case", to_snake_case);
        env.add_filter("table_case", to_table_case);
        env.add_filter("train_case", to_train_case);
        env.add_filter("plural", to_plural);
        env.add_filter("singular", to_singular);
        env.add_filter("foreign_key", to_foreign_key);
        env.add_filter("regex", regex_filter);

        Self { env, default_context }
    }

    /// Internal helper to render templates
    fn render_internal(
        &self,
        template: &str,
        context: &serde_json::Value,
    ) -> Result<String> {
        let mut env = self.env.clone();
        env.add_template("temp", template)?;

        // Merge the default context with the provided context
        let merged_context = if let (Some(default_obj), Some(context_obj)) =
            (self.default_context.as_object(), context.as_object())
        {
            let mut result = default_obj.clone();
            for (key, value) in context_obj {
                result.insert(key.clone(), value.clone());
            }
            json!(result)
        } else {
            // If either isn't an object, just use the provided context
            context.clone()
        };

        let tmpl = env.get_template("temp")?;
        Ok(tmpl.render(merged_context)?)
    }
}

impl Default for MiniJinjaRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateRenderer for MiniJinjaRenderer {
    fn add_template(
        &mut self,
        name: &str,
        template: &str,
    ) -> Result<(), minijinja::Error> {
        self.env.add_template_owned(name.to_string(), template.to_string())
    }

    fn render(&self, template: &str, context: &serde_json::Value) -> Result<String> {
        self.render_internal(template, context)
    }

    fn render_path(
        &self,
        template_path: &Path,
        context: &serde_json::Value,
    ) -> Result<String> {
        let path_str = path_to_str(template_path)?;
        self.render_internal(path_str, context)
    }

    fn execute_expression(
        &self,
        expr_str: &str,
        context: &serde_json::Value,
    ) -> Result<bool> {
        // Only compile the expression if it's not empty
        if expr_str.is_empty() {
            return Ok(true);
        }
        let expr = self.env.compile_expression(expr_str)?;
        Ok(expr.eval(context)?.is_true())
    }
}
