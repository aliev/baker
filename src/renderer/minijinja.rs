use super::filters::*;
use crate::{error::Result, ext::PathExt, renderer::interface::TemplateRenderer};
use minijinja::Environment;
use serde_json::json;
use std::path::Path;

/// MiniJinja-based template rendering engine.
pub struct MiniJinjaRenderer {
    /// MiniJinja environment instance
    env: Environment<'static>,
    /// Default context that will be merged with any provided context
    default_context: serde_json::Value,
}

impl MiniJinjaRenderer {
    /// Creates a new MiniJinjaRenderer instance with default environment.
    pub fn new() -> Self {
        let mut env = Environment::new();
        let default_context = json!({
            "platform": {
                "os": std::env::consts::OS,
                "family": std::env::consts::FAMILY,
                "arch": std::env::consts::ARCH,
            }
        });

        // Add all the custom filters
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

    /// Internal helper to render templates with context merging
    fn render_internal(
        &self,
        template: &str,
        context: &serde_json::Value,
        template_name: Option<&str>,
    ) -> Result<String> {
        let mut env = self.env.clone();
        let name = template_name.unwrap_or("temp");
        env.add_template(name, template)?;

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

        let tmpl = env.get_template(name)?;
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
        // Normalize the template name for cross-platform compatibility
        let normalized_name = name.replace("\\", "/");
        self.env.add_template_owned(normalized_name, template.to_string())
    }

    fn render(
        &self,
        template: &str,
        context: &serde_json::Value,
        template_name: Option<&str>,
    ) -> Result<String> {
        self.render_internal(template, context, template_name)
    }

    fn render_path(
        &self,
        template_path: &Path,
        context: &serde_json::Value,
    ) -> Result<String> {
        let path_str = template_path.to_str_checked()?;
        let template_name = template_path.file_name().and_then(|name| name.to_str());
        self.render_internal(path_str, context, template_name)
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

#[cfg(test)]
mod tests {
    use crate::renderer::{interface::TemplateRenderer, MiniJinjaRenderer};
    use serde_json::json;

    fn test_template(template: &str, expected: &str) {
        let renderer = MiniJinjaRenderer::new();
        let result = renderer.render(template, &json!({}), None).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_string_conversion_filters() {
        test_template("{{ 'hello world' | camel_case }}", "helloWorld");
        test_template("{{ 'hello world' | kebab_case }}", "hello-world");
        test_template("{{ 'hello world' | pascal_case }}", "HelloWorld");
        test_template("{{ 'hello world' | screaming_snake_case }}", "HELLO_WORLD");
        test_template("{{ 'hello world' | snake_case }}", "hello_world");
        test_template("{{ 'Hello World' | table_case }}", "hello_worlds");
        test_template("{{ 'hello world' | train_case }}", "Hello-World");
        test_template("{{ 'car' | plural }}", "cars");
        test_template("{{ 'cars' | singular }}", "car");
        test_template("{{ 'User' | foreign_key }}", "user_id");
        test_template("{{ 'Order Item' | foreign_key }}", "order_item_id");
        test_template("{{ 'orderItem' | foreign_key }}", "order_item_id");
        test_template("{{ 'OrderItem' | foreign_key }}", "order_item_id");
        test_template("{{ 'order_item' | foreign_key }}", "order_item_id");
        test_template("{{ 'order-item' | foreign_key }}", "order_item_id");
        test_template("{{ 'ORDER' | foreign_key }}", "order_id");
        test_template("{{ 'OrderITEM' | foreign_key }}", "order_item_id");
    }

    #[test]
    fn test_regex_filter() {
        test_template("{{ 'hello world' | regex('^hello') }}", "true");
        test_template("{{ 'hello world' | regex('^hello.*') }}", "true");
        test_template("{{ 'goodbye world' | regex('^hello.*') }}", "false");

        test_template("{{ 'Hello World' | regex('hello') }}", "false");
        test_template("{{ 'Hello World' | regex('(?i)hello') }}", "true");

        test_template(r"{{ 'a+b=c' | regex('\\+') }}", "true");
        test_template(r"{{ 'a+b=c' | regex('\\=') }}", "true");
        test_template("{{ 'a+b=c' | regex('d') }}", "false");

        test_template("{{ '' | regex('.*') }}", "true");
        test_template("{{ '' | regex('.+') }}", "false");
        test_template("{{ 'hello' | regex('[') }}", "false");
    }
}
