use crate::error::Result;
use crate::loader::interface::TemplateLoader;
use crate::loader::{git::GitLoader, local::LocalLoader};
use crate::metadata::TemplateMetadata;
use std::path::PathBuf;

pub mod git;
pub mod interface;
pub mod local;

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

/// Creates a TemplateFactory from a string path or URL and loads the template.
///
/// # Arguments
/// * `s` - String containing path or git URL
/// * `skip_overwrite_check` - Whether to skip confirmation for overwriting existing directories
///
/// # Returns
/// * `Result<PathBuf>` - Path to the loaded template
pub fn get_template(
    s: &str,
    skip_overwrite_check: bool,
) -> Result<(PathBuf, TemplateMetadata)> {
    // Check if this is a git repository URL
    let source = if GitLoader::<&str>::is_git_url(s) {
        TemplateSource::Git(s.to_string())
    } else {
        TemplateSource::FileSystem(PathBuf::from(s))
    };

    match source {
        TemplateSource::Git(repo) => {
            GitLoader::new(repo.clone(), skip_overwrite_check).load()
        }
        TemplateSource::FileSystem(path) => LocalLoader::new(path.clone()).load(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_source_display() {
        let fs_source = TemplateSource::FileSystem(PathBuf::from("/path/to/template"));
        assert_eq!(format!("{fs_source}"), "local path: '/path/to/template'");

        let git_source = TemplateSource::Git("git@github.com:user/repo".to_string());
        assert_eq!(format!("{git_source}"), "git repository: 'git@github.com:user/repo'");
    }
}
