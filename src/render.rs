use minijinja::Environment;

use crate::error::{BakerError, BakerResult};

pub trait TemplateRenderer {
    // Processes template content.
    fn render(&self, template: &str, context: &serde_json::Value) -> BakerResult<String>;
}

pub struct MiniJinjaTemplateProcessor {
    env: Environment<'static>,
}
impl MiniJinjaTemplateProcessor {
    pub fn new() -> Self {
        let env = Environment::new();
        Self { env }
    }
}
impl TemplateRenderer for MiniJinjaTemplateProcessor {
    fn render(&self, template: &str, context: &serde_json::Value) -> BakerResult<String> {
        let mut env = self.env.clone();
        env.add_template("temp", template)
            .map_err(|e| BakerError::TemplateError(e.to_string()))?;

        let tmpl = env
            .get_template("temp")
            .map_err(|e| BakerError::TemplateError(e.to_string()))?;

        tmpl.render(context)
            .map_err(|e| BakerError::TemplateError(e.to_string()))
    }
}
