use std::path::PathBuf;

use crate::error::{BakerError, BakerResult};

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
    fn process(&self, template_source: TemplateSource) -> BakerResult<()>;
}

pub struct LocalTemplateProcessor {}

impl LocalTemplateProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl TemplateSourceProcessor for LocalTemplateProcessor {
    fn process(&self, template_source: TemplateSource) -> BakerResult<()> {
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

        Ok(())
    }
}

pub struct GithubTemplateProcessor {}
impl GithubTemplateProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl TemplateSourceProcessor for GithubTemplateProcessor {
    fn process(&self, template_source: TemplateSource) -> BakerResult<()> {
        todo!("this method is not implemented yet")
    }
}
