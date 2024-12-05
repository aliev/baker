//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.

use crate::error::{BakerError, BakerResult};
use indexmap::IndexMap;
use log::debug;
use serde::Deserialize;
use std::path::Path;

/// Supported configuration file names
pub const CONFIG_FILES: [&str; 3] = ["baker.json", "baker.yml", "baker.yaml"];

/// Type of question to be presented to the user
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestionType {
    /// String input question type
    Str,
    /// Boolean (yes/no) question type
    Bool,
}

/// Represents a single question in the configuration
#[derive(Debug, Deserialize)]
pub struct Question {
    /// Help text/prompt to display to the user
    #[serde(default)]
    pub help: String,
    /// Type of the question (string or boolean)
    #[serde(rename = "type")]
    pub question_type: QuestionType,
    /// Optional default value for the question
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    /// Available choices for string questions
    #[serde(default)]
    pub choices: Vec<String>,
    /// Available option for string questions
    #[serde(default)]
    pub multiselect: bool,
    /// Whether the string is a secret
    #[serde(default)]
    pub secret: bool,
    /// Whether the secret should have confirmation
    #[serde(default)]
    pub secret_confirmation: bool,
}

/// Main configuration structure holding all questions
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Map of question identifiers to their configurations
    #[serde(flatten)]
    pub questions: IndexMap<String, Question>,
}

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
pub fn load_config<P: AsRef<Path>>(
    template_dir: P,
    config_files: &[&str],
) -> BakerResult<String> {
    for file in config_files {
        let config_path = template_dir.as_ref().join(file);
        if config_path.exists() {
            debug!("Loading configuration from '{}'.", config_path.display());
            return std::fs::read_to_string(&config_path).map_err(BakerError::IoError);
        }
    }

    Err(BakerError::ConfigError(format!(
        "No configuration file found in '{}'. Tried: {}.",
        template_dir.as_ref().display(),
        config_files.join(", ")
    )))
}
