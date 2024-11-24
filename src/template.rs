use crate::error::{BakerError, BakerResult};
use minijinja::Environment;
use std::path::PathBuf;

#[derive(Debug)]
pub enum TemplateSource {
    LocalPath(PathBuf),
    GitHub(String),
}

impl TemplateSource {
    pub fn from_string(s: &str) -> Option<Self> {
        if s.starts_with("gh@") {
            Some(Self::GitHub(s[3..].to_string()))
        } else {
            let path = PathBuf::from(s);
            Some(Self::LocalPath(path))
        }
    }
}

pub trait TemplateSourceProcessor {
    // Processes template source and returns a local path to it.
    fn process(&self, template_source: TemplateSource) -> BakerResult<PathBuf>;
}

pub trait TemplateProcessor {
    // Processes template content.
    fn process(&self, template: &str, context: &serde_json::Value) -> BakerResult<String>;
}

pub struct LocalTemplateSourceProcessor {}

impl LocalTemplateSourceProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl TemplateSourceProcessor for LocalTemplateSourceProcessor {
    fn process(&self, template_source: TemplateSource) -> BakerResult<PathBuf> {
        let path = match template_source {
            TemplateSource::LocalPath(path) => path,
            // This panic is safe because the `run` function ensures all possible TemplateSource
            // variants, otherwise it returns an error.
            _ => panic!("Expected LocalPath variant"),
        };
        if !path.exists() {
            return Err(BakerError::TemplateError(
                "template path does not exist".to_string(),
            ));
        }

        Ok(path)
    }
}

pub struct GithubTemplateSourceProcessor {}
impl GithubTemplateSourceProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl TemplateSourceProcessor for GithubTemplateSourceProcessor {
    fn process(&self, template_source: TemplateSource) -> BakerResult<PathBuf> {
        todo!("this method is not implemented yet")
    }
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
impl TemplateProcessor for MiniJinjaTemplateProcessor {
    fn process(&self, template: &str, context: &serde_json::Value) -> BakerResult<String> {
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
