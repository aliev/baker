use crate::error::Result;
use crate::loader::interface::TemplateLoader;
use crate::loader::{factory::TemplateSource, git::GitLoader, local::LocalLoader};
use std::path::PathBuf;

pub mod factory;
pub mod git;
pub mod interface;
pub mod local;

/// Creates a TemplateFactory from a string path or URL and loads the template.
///
/// # Arguments
/// * `s` - String containing path or git URL
/// * `skip_overwrite_check` - Whether to skip confirmation for overwriting existing directories
///
/// # Returns
/// * `Result<PathBuf>` - Path to the loaded template
pub fn get_template(s: &str, skip_overwrite_check: bool) -> Result<PathBuf> {
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
