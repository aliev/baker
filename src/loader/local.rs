use crate::error::{Error, Result};
use crate::loader::interface::TemplateLoader;
use crate::metadata::TemplateMetadata;
use std::path::PathBuf;

/// Loader for templates from the local filesystem.
pub struct LocalLoader<P: AsRef<std::path::Path>> {
    path: P,
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
    /// # Returns
    /// * `Result<PathBuf>` - Path to the template directory
    fn load(&self) -> Result<(PathBuf, TemplateMetadata)> {
        let path = self.path.as_ref();
        if !path.exists() {
            return Err(Error::TemplateDoesNotExistsError {
                template_dir: path.display().to_string(),
            });
        }
        let metadata = TemplateMetadata {
            directory: Some(path.to_string_lossy().to_string()),
            ..Default::default()
        };

        Ok((path.to_path_buf(), metadata))
    }
}
