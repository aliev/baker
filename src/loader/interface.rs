use crate::{error::Result, metadata::TemplateMetadata};
use std::path::PathBuf;

/// Trait for loading templates from different sources.
pub trait TemplateLoader {
    /// Loads a template from the given source.
    ///
    /// # Returns
    /// * `Result<PathBuf>` - Path to the loaded template
    fn load(&self) -> Result<(PathBuf, TemplateMetadata)>;
}
