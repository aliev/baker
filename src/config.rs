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

/// Type of question to be presented to the user
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum QuestionType {
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
    help: String,
    /// Type of the question (string or boolean)
    #[serde(rename = "type")]
    question_type: QuestionType,
    /// Optional default value for the question
    #[serde(default)]
    default: Option<serde_json::Value>,
    /// Available choices for string questions
    #[serde(default)]
    choices: Vec<String>,
}

/// Main configuration structure holding all questions
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Map of question identifiers to their configurations
    #[serde(flatten)]
    pub questions: IndexMap<String, Question>,
}

fn parse_str(
    prompt: String,
    key: String,
    engine: &Box<dyn TemplateEngine>,
    question: Question,
    current_context: serde_json::Value,
) -> BakerResult<(String, serde_json::Value)> {
    if !question.choices.is_empty() {
        let selection = Select::new()
            .with_prompt(prompt)
            .default(1)
            .items(&question.choices)
            .interact()
            .map_err(|e| BakerError::ConfigError(e.to_string()))?;

        Ok((
            key,
            serde_json::Value::String(question.choices[selection].clone()),
        ))
    } else {
        let default_value = if let Some(default_value) = question.default {
            if let Some(s) = default_value.as_str() {
                engine.render(s, &current_context).unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let input = Input::new()
            .with_prompt(prompt)
            .default(default_value)
            .interact_text()
            .map_err(|e| BakerError::ConfigError(e.to_string()))?;

        Ok((key, serde_json::Value::String(input)))
    }
}

fn parse_bool(
    prompt: String,
    key: String,
    question: Question,
) -> BakerResult<(String, serde_json::Value)> {
    let default_value = question.default.and_then(|v| v.as_bool()).unwrap_or(false);

    let result = Confirm::new()
        .with_prompt(prompt)
        .default(default_value)
        .interact()
        .map_err(|e| BakerError::ConfigError(e.to_string()))?;

    Ok((key, serde_json::Value::Bool(result)))
}

/// Prompts the user for answers to all configured questions
///
/// # Arguments
/// * `questions` - Map of questions to ask
/// * `engine` - Template engine for rendering help text and default values
///
/// # Returns
/// * `BakerResult<serde_json::Value>` - JSON object containing all answers
///
/// # Errors
/// * `BakerError::ConfigError` if there's an error during user interaction
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
                let (key, value) = parse_str(prompt, key, engine, question, current_context)?;
                context.insert(key, value);
            }
            QuestionType::Bool => {
                let (key, value) = parse_bool(prompt, key, question)?;

                context.insert(key, value);
            }
        };
    }

    Ok(serde_json::Value::Object(context))
}
