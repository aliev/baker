//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.

use crate::error::{BakerError, BakerResult};
use crate::template::TemplateEngine;
use indexmap::IndexMap;
use std::path::Path;

/// Loads configuration from a JSON file at the specified path.
///
/// # Arguments
/// * `config_path` - Path to the configuration file
///
/// # Returns
/// * `BakerResult<String>` - Contents of the configuration file as a string
///
/// # Errors
/// * `BakerError::IoError` if the file cannot be read
pub fn load_config<P: AsRef<Path>>(config_path: P) -> BakerResult<String> {
    if !config_path.as_ref().exists() {
        return Err(BakerError::ConfigError(
            "Configuration file does not exist".to_string(),
        ));
    }

    Ok(std::fs::read_to_string(config_path).map_err(BakerError::IoError)?)
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
/// * `BakerResult<IndexMap<String, serde_json::Value>>` - Processed configuration as key-value pairs
///
/// # Errors
/// * `BakerError::ConfigError` if JSON parsing fails
pub fn parse_config(
    content: String,
    engine: &Box<dyn TemplateEngine>,
) -> BakerResult<IndexMap<String, serde_json::Value>> {
    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| BakerError::ConfigError(format!("Invalid JSON: {}", e)))?;

    if let serde_json::Value::Object(map) = value {
        let mut result = IndexMap::new();
        for (key, value) in map {
            result.insert(key, process_config_value(&value, &value, engine)?);
        }
        Ok(result)
    } else {
        Err(BakerError::ConfigError(
            "Configuration must be a JSON object".to_string(),
        ))
    }
}
