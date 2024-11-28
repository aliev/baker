//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.

use crate::error::{BakerError, BakerResult};
use crate::template::TemplateEngine;
use dialoguer::{Confirm, Input, Select};
use indexmap::IndexMap;
use log::debug;
use serde::Deserialize;
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

pub fn parse_config(content: String) -> BakerResult<IndexMap<String, serde_json::Value>> {
    let raw_value: IndexMap<String, serde_json::Value> = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => serde_yaml::from_str(&content)
            .map_err(|e| BakerError::ConfigError(format!("Invalid configuration format: {}", e)))?,
    };
    Ok(raw_value)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum QuestionType {
    Str,
    Bool,
}

#[derive(Debug, Deserialize)]
pub struct Question {
    #[serde(default)]
    help: String,
    #[serde(rename = "type")]
    question_type: QuestionType,
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    choices: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub questions: IndexMap<String, Question>,
}

pub fn prompt_questions(
    questions: IndexMap<String, Question>,
    engine: &Box<dyn TemplateEngine>,
) -> BakerResult<serde_json::Value> {
    let mut context = serde_json::Map::new();

    for (key, question) in questions {
        let current_context = serde_json::Value::Object(context.clone());
        let prompt = engine
            .render(&question.help, &current_context)
            .unwrap_or(question.help.clone());

        match question.question_type {
            QuestionType::Str => {
                if !question.choices.is_empty() {
                    let selection = Select::new()
                        .with_prompt(prompt)
                        .default(0)
                        .items(&question.choices)
                        .interact()
                        .map_err(|e| BakerError::ConfigError(e.to_string()))?;

                    context.insert(
                        key,
                        serde_json::Value::String(question.choices[selection].clone()),
                    );
                } else {
                    let default_value = if let Some(default_template) = question.default {
                        engine
                            .render(&default_template, &current_context)
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };

                    let input: String = Input::new()
                        .with_prompt(prompt)
                        .default(default_value)
                        .interact_text()
                        .map_err(|e| BakerError::ConfigError(e.to_string()))?;

                    context.insert(key, serde_json::Value::String(input));
                }
            }
            QuestionType::Bool => {
                let default_value = question
                    .default
                    .and_then(|v| match v.to_lowercase().as_str() {
                        "yes" | "true" | "1" => Some(true),
                        "no" | "false" | "0" => Some(false),
                        _ => None,
                    })
                    .unwrap_or(false);

                let result = Confirm::new()
                    .with_prompt(prompt)
                    .default(default_value)
                    .interact()
                    .map_err(|e| BakerError::ConfigError(e.to_string()))?;

                context.insert(key, serde_json::Value::Bool(result));
            }
        };
    }

    Ok(serde_json::Value::Object(context))
}
