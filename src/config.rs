//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.

use crate::error::{BakerError, BakerResult};
use crate::template::TemplateEngine;
use indexmap::IndexMap;
use log::debug;
use std::path::Path;

/// Supported configuration file names
pub const CONFIG_FILES: [&str; 3] = ["baker.json", "baker.yml", "baker.yaml"];

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
/// * `BakerResult<IndexMap<String, serde_json::Value>>` - Processed configuration
///
/// # Errors
/// * `BakerError::ConfigError` if parsing fails
pub fn parse_config(
    content: String,
    engine: &Box<dyn TemplateEngine>,
) -> BakerResult<IndexMap<String, serde_json::Value>> {
    // Try parsing as JSON first
    let value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => {
            // If JSON fails, try YAML
            serde_yaml::from_str(&content).map_err(|e| {
                BakerError::ConfigError(format!("Invalid configuration format: {}", e))
            })?
        }
    };

    if let serde_json::Value::Object(map) = value {
        let mut result = IndexMap::new();
        for (key, value) in map {
            result.insert(key, process_config_value(&value, &value, engine)?);
        }
        Ok(result)
    } else {
        Err(BakerError::ConfigError(
            "Configuration must be an object".to_string(),
        ))
    }
}
