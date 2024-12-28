//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.

use crate::{
    error::{Error, Result},
    question::Question,
};
use indexmap::IndexMap;
use log::debug;
use serde::Deserialize;
use std::path::Path;

/// Supported configuration file names
pub const CONFIG_FILES: [&str; 3] = ["baker.json", "baker.yml", "baker.yaml"];

/// Main configuration structure holding all questions
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Map of question identifiers to their configurations
    #[serde(flatten)]
    pub questions: IndexMap<String, Question>,
}

impl Config {
    /// Loads configuration from a template directory, trying multiple file formats.
    /// Supports: baker.json, baker.yml, baker.yaml
    ///
    /// # Arguments
    /// * `template_dir` - Directory containing the template configuration
    /// * `config_files` - List of configuration files to try
    ///
    /// # Returns
    /// * `BakerResult<String>` - Contents of the first found configuration file
    ///
    /// # Errors
    /// * `BakerError::ConfigError` if no valid config file exists
    fn load_config<P: AsRef<Path>>(
        template_dir: P,
        config_files: &[&str],
    ) -> Result<String> {
        for file in config_files {
            let config_path = template_dir.as_ref().join(file);
            if config_path.exists() {
                debug!("Loading configuration from '{}'.", config_path.display());
                return std::fs::read_to_string(&config_path).map_err(Error::IoError);
            }
        }

        Err(Error::ConfigError {
            template_dir: template_dir.as_ref().display().to_string(),
            config_files: config_files.join(", "),
        })
    }

    /// Parses config file.
    fn parse_config<S: Into<String>>(config_content: S) -> Result<Config> {
        let config_content: String = config_content.into();
        let config: Config =
            serde_yaml::from_str(&config_content).map_err(Error::ConfigParseError)?;
        Ok(config)
    }

    pub fn from_file(template_root: &Path) -> Result<Self> {
        let config_content = Self::load_config(template_root, &CONFIG_FILES)?;
        Self::parse_config(config_content)
    }
}
