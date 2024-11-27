//! Template loading and rendering functionality for Baker.
//! Handles both local filesystem and git repository templates with support
//! for MiniJinja template processing.
use crate::error::{BakerError, BakerResult};
use git2::{Cred, FetchOptions, RemoteCallbacks};
use minijinja::Environment;
use std::env;
use std::path::PathBuf;
use tempfile::TempDir;
use url::Url;

/// Represents the source location of a template.
#[derive(Debug)]
pub enum TemplateSource {
    /// Local filesystem template path
    FileSystem(PathBuf),
    /// Git repository URL (HTTPS or SSH)
    Git(String),
}

impl TemplateSource {
    /// Creates a TemplateSource from a string path or URL.
    ///
    /// # Arguments
    /// * `s` - String containing path or git URL
    ///
    /// # Returns
    /// * `Option<Self>` - Some(TemplateSource) if valid input
    ///
    /// # Examples
    /// ```
    /// use baker::template::TemplateSource;
    /// let local = TemplateSource::from_string("./templates/web");
    /// let git = TemplateSource::from_string("https://github.com/user/template.git");
    /// ```
    pub fn from_string(s: &str) -> Option<Self> {
        // First try to parse as URL
        if let Ok(url) = Url::parse(s) {
            if url.scheme() == "https" || url.scheme() == "git" {
                return Some(Self::Git(s.to_string()));
            }
        }

        // Check for SSH git URL format
        if s.starts_with("git@") {
            return Some(Self::Git(s.to_string()));
        }

        // Treat as filesystem path
        let path = PathBuf::from(s);
        Some(Self::FileSystem(path))
    }
}

/// Trait for loading templates from different sources.
pub trait TemplateLoader {
    /// Loads a template from the given source.
    ///
    /// # Arguments
    /// * `source` - Source location of the template
    ///
    /// # Returns
    /// * `BakerResult<PathBuf>` - Path to the loaded template
    fn load(&self, source: &TemplateSource) -> BakerResult<PathBuf>; // was process
}

/// Trait for template rendering engines.
pub trait TemplateEngine {
    /// Renders a template string with the given context.
    ///
    /// # Arguments
    /// * `template` - Template string to render
    /// * `context` - Context variables for rendering
    ///
    /// # Returns
    /// * `BakerResult<String>` - Rendered template string
    fn render(&self, template: &str, context: &serde_json::Value) -> BakerResult<String>;
}

/// Loader for templates from the local filesystem.
pub struct LocalLoader {}
/// Loader for templates from git repositories.
pub struct GitLoader {
    _temp_dir: Option<TempDir>, // Keep TempDir alive
}
/// MiniJinja-based template rendering engine.
pub struct MiniJinjaEngine {
    /// MiniJinja environment instance
    env: Environment<'static>,
}

impl LocalLoader {
    /// Creates a new LocalLoader instance.
    pub fn new() -> Self {
        Self {}
    }
}

impl TemplateLoader for LocalLoader {
    /// Loads a template from the local filesystem.
    ///
    /// # Arguments
    /// * `source` - Template source (must be FileSystem variant)
    ///
    /// # Returns
    /// * `BakerResult<PathBuf>` - Path to the template directory
    ///
    /// # Errors
    /// * `BakerError::TemplateError` if path doesn't exist
    /// * Panics if source is not FileSystem variant
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
    /// Creates a new GitLoader instance.
    pub fn new() -> Self {
        Self { _temp_dir: None }
    }
}

impl TemplateLoader for GitLoader {
    /// Loads a template by cloning a git repository.
    ///
    /// # Arguments
    /// * `source` - Template source (must be Git variant)
    ///
    /// # Returns
    /// * `BakerResult<PathBuf>` - Path to the cloned repository
    ///
    /// # Errors
    /// * `BakerError::TemplateError` if:
    ///   - Source is not Git variant
    ///   - Temp directory creation fails
    ///   - Repository clone fails
    ///
    /// # Notes
    /// - Clones to system temp directory under 'baker-templates'
    /// - Uses SSH authentication with default key location
    fn load(&self, source: &TemplateSource) -> BakerResult<PathBuf> {
        let repo_url = match source {
            TemplateSource::Git(url) => url,
            _ => return Err(BakerError::TemplateError("Expected Git URL".to_string())),
        };

        let temp_dir = tempfile::Builder::new()
            .prefix("baker-")
            .tempdir()
            .map_err(|e| BakerError::TemplateError(format!("Failed to create temp dir: {}", e)))?;

        let repo_name = repo_url
            .split('/')
            .last()
            .unwrap_or("temp")
            .trim_end_matches(".git");
        let clone_path = temp_dir.path().join(repo_name);

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
    /// Creates a new MiniJinjaEngine instance with default environment.
    pub fn new() -> Self {
        let env = Environment::new();
        Self { env }
    }
}

impl TemplateEngine for MiniJinjaEngine {
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
