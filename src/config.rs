//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.

use crate::error::{BakerError, BakerResult};
use crate::template::TemplateEngine;
use indexmap::IndexMap;
use log::debug;
use serde::Deserialize;
use std::path::Path;

/// Supported configuration file names
pub const CONFIG_FILES: [&str; 3] = ["baker.json", "baker.yml", "baker.yaml"];

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ConfigValue {
    #[serde(rename = "string")]
    String { question: String, default: String },
    #[serde(rename = "boolean")]
    Boolean { question: String, default: bool },
    #[serde(rename = "array")]
    Array {
        question: String,
        choices: Vec<String>,
    },
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
pub fn load_config<P: AsRef<Path>>(template_dir: P, config_files: &[&str]) -> BakerResult<String> {
    for file in config_files {
        let config_path = template_dir.as_ref().join(file);
        if config_path.exists() {
            debug!("Loading configuration from {}", config_path.display());
            return Ok(std::fs::read_to_string(&config_path).map_err(BakerError::IoError)?);
        }
    }

    Err(BakerError::ConfigError(format!(
        "No configuration file found (tried: {})",
        config_files.join(", ")
    )))
}

/// Processes a configuration value, recursively handling template interpolation.
///
/// # Arguments
/// * `value` - The JSON value to process
/// * `context` - Template context for variable interpolation
/// * `engine` - Template engine for rendering
///
/// # Returns
/// * `BakerResult<serde_json::Value>` - Processed JSON value with interpolated variables
///
/// # Note
/// Handles three types of values:
/// - Strings: Processed as templates
/// - Arrays: Each element processed recursively
/// - Objects: Each value processed recursively
fn process_config_value(
    value: &serde_json::Value,
    context: &serde_json::Value,
    engine: &Box<dyn TemplateEngine>,
) -> BakerResult<serde_json::Value> {
    match value {
        serde_json::Value::String(s) => {
            // Process string values as templates
            let processed = engine.render(s, context)?;
            Ok(serde_json::Value::String(processed))
        }
        serde_json::Value::Array(arr) => {
            // Process each array item recursively
            let mut processed_arr = Vec::new();
            for item in arr {
                processed_arr.push(process_config_value(item, context, engine)?);
            }
            Ok(serde_json::Value::Array(processed_arr))
        }
        serde_json::Value::Object(obj) => {
            // Process each object field recursively
            let mut processed_obj = serde_json::Map::new();
            for (k, v) in obj {
                processed_obj.insert(k.clone(), process_config_value(v, context, engine)?);
            }
            Ok(serde_json::Value::Object(processed_obj))
        }
        // Non-string values are returned as-is
        _ => Ok(value.clone()),
    }
}

/// Parses and processes the configuration content.
///
/// # Arguments
/// * `content` - Raw configuration content as string
/// * `engine` - Template engine for rendering
///
/// # Returns
/// * `BakerResult<IndexMap<String, ConfigValue>>` - Processed configuration
///
/// # Errors
/// * `BakerError::ConfigError` if parsing fails
pub fn parse_config(
    content: String,
    engine: &Box<dyn TemplateEngine>,
) -> BakerResult<IndexMap<String, ConfigValue>> {
    // Try parsing as JSON first, explicitly as IndexMap
    let raw_value: IndexMap<String, serde_json::Value> = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => serde_yaml::from_str(&content)
            .map_err(|e| BakerError::ConfigError(format!("Invalid configuration format: {}", e)))?,
    };

    // Process each field in order, building up the context as we go
    let mut context = serde_json::Map::new();
    let mut result = IndexMap::new();

    for (key, value) in raw_value {
        let config_value: ConfigValue = serde_json::from_value(value.clone())
            .map_err(|e| BakerError::ConfigError(format!("Invalid schema: {}", e)))?;

        // Process this field with current context
        let current_context = serde_json::Value::Object(context.clone());
        let processed_value = match config_value {
            ConfigValue::String { question, default } => {
                let processed_question = engine.render(&question, &current_context)?;
                let processed_default = engine.render(&default, &current_context)?;
                ConfigValue::String {
                    question: processed_question,
                    default: processed_default,
                }
            }
            ConfigValue::Boolean { question, default } => {
                let processed_question = engine.render(&question, &current_context)?;
                ConfigValue::Boolean {
                    question: processed_question,
                    default,
                }
            }
            ConfigValue::Array { question, choices } => {
                let processed_question = engine.render(&question, &current_context)?;
                let processed_choices = choices
                    .into_iter()
                    .map(|c| engine.render(&c, &current_context))
                    .collect::<Result<Vec<_>, _>>()?;
                ConfigValue::Array {
                    question: processed_question,
                    choices: processed_choices,
                }
            }
        };

        // Add this field to context for next iterations
        context.insert(key.clone(), value);
        result.insert(key, processed_value);
    }

    Ok(result)
}
