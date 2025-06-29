use crate::error::Result;
use crate::loader::git::GitLoader;
use crate::loader::interface::TemplateLoader;
use crate::loader::local::LocalLoader;
use std::path::PathBuf;

pub mod git;
pub mod interface;
pub mod local;

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
            TemplateSource::Git(repo) => write!(f, "git repository: '{repo}'"),
        }
    }
}

impl TemplateSource {
    /// Creates a TemplateSource from a string path or URL.
    ///
    /// # Arguments
    /// * `s` - String containing path or git URL
    /// * `skip_overwrite_check` - Whether to skip confirmation for overwriting existing directories
    ///
    /// # Returns
    /// * `Result<PathBuf>` - Path to the loaded template
    pub fn load_from_string(s: &str, skip_overwrite_check: bool) -> Result<PathBuf> {
        // Check if this is a git repository URL
        let source = if GitLoader::<&str>::is_git_url(s) {
            Self::Git(s.to_string())
        } else {
            Self::FileSystem(PathBuf::from(s))
        };

        match source {
            TemplateSource::Git(repo) => {
                GitLoader::new(repo, skip_overwrite_check).load()
            }
            TemplateSource::FileSystem(path) => LocalLoader::new(path).load(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_source_display() {
        let fs_source = TemplateSource::FileSystem(PathBuf::from("/path/to/template"));
        assert_eq!(format!("{}", fs_source), "local path: '/path/to/template'");

        let git_source = TemplateSource::Git("git@github.com:user/repo".to_string());
        assert_eq!(
            format!("{}", git_source),
            "git repository: 'git@github.com:user/repo'"
        );
    }
}
