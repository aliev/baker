//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.

use crate::config::{Question, QuestionType};
use crate::error::{BakerError, BakerResult};
use crate::template::TemplateEngine;
use dialoguer::{Confirm, Input, MultiSelect, Password, Select};
use indexmap::IndexMap;

/// Prompts the user for multiple selections from a list of choices.
///
/// # Arguments
/// * `prompt` - The prompt text to display to the user
/// * `key` - The key to associate with the selected values
/// * `question` - The question configuration containing available choices
///
/// # Returns
/// * `BakerResult<(String, serde_json::Value)>` - The key and array of selected values
///
/// # Errors
/// * Returns `BakerError::ConfigError` if user interaction fails
fn prompt_multi_selection(
    prompt: String,
    key: String,
    question: Question,
) -> BakerResult<(String, serde_json::Value)> {
    let indices = MultiSelect::new()
        .with_prompt(prompt)
        .items(&question.choices)
        .interact()
        .map_err(|e| {
            BakerError::ConfigError(format!("failed to get user selection: {}", e))
        })?;

    let selected: Vec<serde_json::Value> = indices
        .iter()
        .map(|&i| serde_json::Value::String(question.choices[i].clone()))
        .collect();

    Ok((key, serde_json::Value::Array(selected)))
}

/// Prompts the user to select a single item from a list of choices.
///
/// # Arguments
/// * `prompt` - The prompt text to display to the user
/// * `key` - The key to associate with the selected value
/// * `question` - The question configuration containing available choices and default value
///
/// # Returns
/// * `BakerResult<(String, serde_json::Value)>` - The key and selected value
///
/// # Errors
/// * Returns `BakerError::ConfigError` if user interaction fails
fn prompt_selection(
    prompt: String,
    key: String,
    question: Question,
) -> BakerResult<(String, serde_json::Value)> {
    let default_value = if let Some(default_value) = question.default {
        if let Some(default_str) = default_value.as_str() {
            question.choices.iter().position(|choice| choice == default_str).unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    };

    let selection = Select::new()
        .with_prompt(prompt)
        .default(default_value)
        .items(&question.choices)
        .interact()
        .map_err(|e| {
            BakerError::ConfigError(format!("failed to get user selection: {}", e))
        })?;

    Ok((key, serde_json::Value::String(question.choices[selection].clone())))
}

/// Prompts the user for a string input with optional default value and secret handling.
///
/// # Arguments
/// * `prompt` - The prompt text to display to the user
/// * `key` - The key to associate with the input value
/// * `engine` - Template engine for rendering default values
/// * `default` - Optional default value that may contain template variables
/// * `current_context` - Current context for template variable interpolation
/// * `is_secret` - Whether to handle the input as a password/secret
/// * `is_secret_confirmation` - Whether to require confirmation for secret input
///
/// # Returns
/// * `BakerResult<(String, serde_json::Value)>` - The key and input value
///
/// # Errors
/// * Returns `BakerError::ConfigError` if user interaction fails
fn prompt_string(
    prompt: String,
    key: String,
    engine: &dyn TemplateEngine,
    default: Option<serde_json::Value>,
    current_context: serde_json::Value,
    is_secret: bool,
    is_secret_confirmation: bool,
) -> BakerResult<(String, serde_json::Value)> {
    let default_value = if let Some(default_value) = default {
        if let Some(s) = default_value.as_str() {
            engine.render(s, &current_context).unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let input = if is_secret {
        let mut password = Password::new().with_prompt(&prompt);

        if is_secret_confirmation {
            password =
                password.with_confirmation(format!("{} (confirm)", &prompt), "Mistmatch");
        }

        password.interact().map_err(|e| {
            BakerError::ConfigError(format!("failed to get user input: {}", e))
        })?
    } else {
        Input::new().with_prompt(&prompt).default(default_value).interact_text().map_err(
            |e| BakerError::ConfigError(format!("failed to get user input: {}", e)),
        )?
    };

    Ok((key, serde_json::Value::String(input)))
}

/// Prompts the user for a boolean (yes/no) response.
///
/// # Arguments
/// * `prompt` - The prompt text to display to the user
/// * `key` - The key to associate with the boolean value
/// * `question` - The question configuration containing the default value
///
/// # Returns
/// * `BakerResult<(String, serde_json::Value)>` - The key and boolean value
///
/// # Errors
/// * Returns `BakerError::ConfigError` if user interaction fails
fn prompt_bool(
    prompt: String,
    key: String,
    question: Question,
) -> BakerResult<(String, serde_json::Value)> {
    let default_value = question.default.and_then(|v| v.as_bool()).unwrap_or(false);

    let result =
        Confirm::new().with_prompt(prompt).default(default_value).interact().map_err(
            |e| {
                BakerError::ConfigError(format!("failed to get user confirmation: {}", e))
            },
        )?;

    Ok((key, serde_json::Value::Bool(result)))
}

/// Prompts for confirmation before executing hooks.
///
/// # Arguments
/// * `skip_hooks_check` - Whether to skip the confirmation prompt
///
/// # Returns
/// * `BakerResult<bool>` - Whether hooks should be executed
///
/// # Safety
/// This function provides a safety check before executing potentially dangerous hook scripts.
pub fn prompt_confirm_hooks_execution<S: Into<String>>(
    skip_hooks_check: bool,
    prompt: S,
) -> BakerResult<bool> {
    if skip_hooks_check {
        return Ok(true);
    }
    Confirm::new().with_prompt(prompt).default(false).interact().map_err(|e| {
        BakerError::HookError(format!("failed to get hooks confirmation: {}", e))
    })
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
    engine: &dyn TemplateEngine,
) -> BakerResult<serde_json::Value> {
    let mut context = serde_json::Map::new();

    for (key, question) in questions {
        let current_context = serde_json::Value::Object(context.clone());
        let prompt = engine
            .render(&question.help, &current_context)
            .unwrap_or(question.help.clone());

        match question.question_type {
            QuestionType::Str => {
                let (key, value) = if !question.choices.is_empty() {
                    if question.multiselect {
                        prompt_multi_selection(prompt, key, question)?
                    } else {
                        prompt_selection(prompt, key, question)?
                    }
                } else {
                    prompt_string(
                        prompt,
                        key,
                        engine,
                        question.default,
                        current_context,
                        question.secret,
                        question.secret_confirmation,
                    )?
                };
                context.insert(key, value);
            }
            QuestionType::Bool => {
                let (key, value) = prompt_bool(prompt, key, question)?;

                context.insert(key, value);
            }
        };
    }

    Ok(serde_json::Value::Object(context))
}
