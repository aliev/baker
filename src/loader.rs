//! Template loading and rendering functionality for Baker.
//! Handles both local filesystem and git repository templates with support
//! for MiniJinja template processing.
use crate::error::{Error, Result};
use crate::prompt::Prompter;
use git2;
use log::debug;
use std::fs;
use std::path::PathBuf;
use url::Url;

/// Represents the source location of a template.
#[derive(Debug)]
pub enum TemplateSource {
    /// Local filesystem template path
    FileSystem(PathBuf),
    /// Git repository URL (HTTPS or SSH)
    Git(String),
}

impl std::fmt::Display for TemplateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateSource::FileSystem(path) => {
                write!(f, "local path: '{}'", path.display())
            }
            TemplateSource::Git(repo) => write!(f, "git repository: '{}'", repo),
        }
    }
}

impl TemplateSource {
    /// Creates a TemplateSource from a string path or URL.
    ///
    /// # Arguments
    /// * `s` - String containing path or git URL
    ///
    /// # Returns
    /// * `Option<Self>` - Some(TemplateSource) if valid input
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
    fn load(&self) -> Result<PathBuf>; // was process
}

/// Loader for templates from the local filesystem.
pub struct LocalLoader<P: AsRef<std::path::Path>> {
    path: P,
}
/// Loader for templates from git repositories.
pub struct GitLoader<'a, S: AsRef<str>> {
    prompt: &'a dyn Prompter,
    repo: S,
    skip_overwrite_check: bool,
}
impl<P: AsRef<std::path::Path>> LocalLoader<P> {
    /// Creates a new LocalLoader instance.
    pub fn new(path: P) -> Self {
        Self { path }
    }
}

impl<P: AsRef<std::path::Path>> TemplateLoader for LocalLoader<P> {
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
    fn load(&self) -> Result<PathBuf> {
        let path = self.path.as_ref();
        if !path.exists() {
            return Err(Error::TemplateDoesNotExistsError {
                template_dir: path.display().to_string(),
            });
        }

        Ok(path.to_path_buf())
    }
}

impl<'a, S: AsRef<str>> GitLoader<'a, S> {
    /// Creates a new GitLoader instance.
    pub fn new(prompt: &'a dyn Prompter, repo: S, skip_overwrite_check: bool) -> Self {
        Self { repo, skip_overwrite_check, prompt }
    }
}

impl<S: AsRef<str>> TemplateLoader for GitLoader<'_, S> {
    /// Loads a template by cloning a git repository.
    ///
    /// # Arguments
    /// * `source` - Template source (must be Git variant)
    ///
    /// # Returns
    /// * `BakerResult<PathBuf>` - Path to the cloned repository
    ///
    /// # Errors
    /// * `BakerError::TemplateError` if clone fails
    fn load(&self) -> Result<PathBuf> {
        let repo_url = self.repo.as_ref();

        debug!("Cloning repository '{}'.", repo_url);

        let repo_name =
            repo_url.split('/').last().unwrap_or("template").trim_end_matches(".git");
        let clone_path = PathBuf::from(repo_name);

        if clone_path.exists() {
            let response = self.prompt.confirm(
                self.skip_overwrite_check,
                format!("Directory '{}' already exists. Replace it?", repo_name),
            )?;
            if response {
                fs::remove_dir_all(&clone_path).map_err(Error::IoError)?;
            } else {
                debug!("Using existing directory '{}'.", clone_path.display());
                return Ok(clone_path);
            }
        }

        debug!("Cloning to '{}'.", clone_path.display());

        // Set up authentication callbacks
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            git2::Cred::ssh_key(
                username_from_url.unwrap_or("git"),
                None,
                std::path::Path::new(&format!(
                    "{}/.ssh/id_rsa",
                    std::env::var("HOME").unwrap()
                )),
                None,
            )
        });

        // Configure fetch options with callbacks
        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        // Set up and perform clone
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_opts);

        match builder.clone(repo_url, &clone_path) {
            Ok(_) => Ok(clone_path),
            Err(e) => Err(Error::Git2Error(e)),
        }
    }
}

/// Returns the template directory from provided template source
pub fn load_template<S: Into<String>>(
    prompt: &dyn Prompter,
    template: S,
    skip_overwrite_check: bool,
) -> Result<PathBuf> {
    let template: String = template.into();
    let template_source = match TemplateSource::from_string(&template) {
        Some(source) => Ok(source),
        None => {
            Err(Error::TemplateError(format!("invalid template source: {}", template)))
        }
    }?;

    println!("Using template from the {}", template_source);

    let loader: Box<dyn TemplateLoader> = match template_source {
        TemplateSource::Git(repo) => {
            Box::new(GitLoader::new(prompt, repo, skip_overwrite_check))
        }
        TemplateSource::FileSystem(path) => Box::new(LocalLoader::new(path)),
    };

    loader.load()
}
