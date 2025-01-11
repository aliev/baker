use crate::dialoguer::confirm;
use crate::error::{Error, Result};
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
    pub fn from_string(s: &str, skip_overwrite_check: bool) -> Result<PathBuf> {
        // First try to parse as URL
        let source = if let Ok(url) = Url::parse(s) {
            if url.scheme() == "https" || url.scheme() == "git" {
                Self::Git(s.to_string())
            } else {
                let path = PathBuf::from(s);
                Self::FileSystem(path)
            }
        } else {
            let path = PathBuf::from(s);
            Self::FileSystem(path)
        };

        let loader: Box<dyn TemplateLoader> = match source {
            TemplateSource::Git(repo) => {
                Box::new(GitLoader::new(repo, skip_overwrite_check))
            }
            TemplateSource::FileSystem(path) => Box::new(LocalLoader::new(path)),
        };

        loader.load()
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
pub struct GitLoader<S: AsRef<str>> {
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

impl<S: AsRef<str>> GitLoader<S> {
    /// Creates a new GitLoader instance.
    pub fn new(repo: S, skip_overwrite_check: bool) -> Self {
        Self { repo, skip_overwrite_check }
    }
}

impl<S: AsRef<str>> TemplateLoader for GitLoader<S> {
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
            let response = confirm(
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
