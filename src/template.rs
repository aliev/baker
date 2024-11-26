use crate::error::{BakerError, BakerResult};
use git2::{Cred, FetchOptions, RemoteCallbacks};
use minijinja::Environment;
use std::env;
use std::path::PathBuf;

#[derive(Debug)]
pub enum TemplateSource {
    FileSystem(PathBuf),
    Git(String),
}

impl TemplateSource {
    pub fn from_string(s: &str) -> Option<Self> {
        if s.starts_with("git@") || s.starts_with("https://") {
            Some(Self::Git(s.to_string()))
        } else {
            let path = PathBuf::from(s);
            Some(Self::FileSystem(path))
        }
    }
}

pub trait TemplateLoader {
    fn load(&self, source: &TemplateSource) -> BakerResult<PathBuf>; // was process
}

pub trait TemplateEngine {
    fn render(&self, template: &str, context: &serde_json::Value) -> BakerResult<String>;
}

pub struct LocalLoader {}
pub struct GitLoader {}
pub struct MiniJinjaEngine {
    env: Environment<'static>,
}

impl LocalLoader {
    pub fn new() -> Self {
        Self {}
    }
}

impl TemplateLoader for LocalLoader {
    fn load(&self, source: &TemplateSource) -> BakerResult<PathBuf> {
        let path = match source {
            TemplateSource::FileSystem(path) => path,
            _ => panic!("Expected LocalPath variant"),
        };
        if !path.exists() {
            return Err(BakerError::TemplateError(
                "template path does not exist".to_string(),
            ));
        }

        Ok(path.to_path_buf())
    }
}

impl GitLoader {
    pub fn new() -> Self {
        Self {}
    }
}

impl TemplateLoader for GitLoader {
    fn load(&self, source: &TemplateSource) -> BakerResult<PathBuf> {
        let repo_url = match source {
            TemplateSource::Git(url) => url,
            _ => return Err(BakerError::TemplateError("Expected Git URL".to_string())),
        };

        let temp_dir = env::temp_dir().join("baker-templates");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| BakerError::TemplateError(format!("Failed to create temp dir: {}", e)))?;

        let repo_name = repo_url
            .split('/')
            .last()
            .unwrap_or("temp")
            .trim_end_matches(".git");
        let clone_path = temp_dir.join(repo_name);

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key(
                username_from_url.unwrap_or("git"),
                None,
                std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
                None,
            )
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        match builder.clone(repo_url, &clone_path) {
            Ok(_) => Ok(clone_path),
            Err(e) => Err(BakerError::TemplateError(format!(
                "Failed to clone repository: {}",
                e
            ))),
        }
    }
}

impl MiniJinjaEngine {
    pub fn new() -> Self {
        let env = Environment::new();
        Self { env }
    }
}

impl TemplateEngine for MiniJinjaEngine {
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
