//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.

use crate::config::{Question, QuestionType};
use crate::error::{BakerError, BakerResult};
use crate::template::TemplateEngine;
use dialoguer::{Confirm, Input, MultiSelect, Select};
use indexmap::IndexMap;

fn prompt_multi_selection(
    prompt: String,
    key: String,
    question: Question,
) -> BakerResult<(String, serde_json::Value)> {
    let indices = MultiSelect::new()
        .with_prompt(prompt)
        .items(&question.choices)
        .interact()
        .map_err(|e| BakerError::ConfigError(format!("failed to get user selection: {}", e)))?;

    let selected: Vec<serde_json::Value> = indices
        .iter()
        .map(|&i| serde_json::Value::String(question.choices[i].clone()))
        .collect();

    Ok((key, serde_json::Value::Array(selected)))
}

fn prompt_selection(
    prompt: String,
    key: String,
    question: Question,
) -> BakerResult<(String, serde_json::Value)> {
    let default_value = if let Some(default_value) = question.default {
        if let Some(default_str) = default_value.as_str() {
            question
                .choices
                .iter()
                .position(|choice| choice == default_str)
                .unwrap_or(0)
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
        .map_err(|e| BakerError::ConfigError(format!("failed to get user selection: {}", e)))?;

    Ok((
        key,
        serde_json::Value::String(question.choices[selection].clone()),
    ))
}

fn prompt_string(
    prompt: String,
    key: String,
    engine: &Box<dyn TemplateEngine>,
    default: Option<serde_json::Value>,
    current_context: serde_json::Value,
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
    let input = Input::new()
        .with_prompt(prompt)
        .default(default_value)
        .interact_text()
        .map_err(|e| BakerError::ConfigError(format!("failed to get user input: {}", e)))?;

    Ok((key, serde_json::Value::String(input)))
}

fn prompt_bool(
    prompt: String,
    key: String,
    question: Question,
) -> BakerResult<(String, serde_json::Value)> {
    let default_value = question.default.and_then(|v| v.as_bool()).unwrap_or(false);

    let result = Confirm::new()
        .with_prompt(prompt)
        .default(default_value)
        .interact()
        .map_err(|e| BakerError::ConfigError(format!("failed to get user confirmation: {}", e)))?;

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
    Ok(Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()
        .map_err(|e| BakerError::HookError(format!("failed to get hooks confirmation: {}", e)))?)
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
                let (key, value) = if !question.choices.is_empty() {
                    if question.multiselect {
                        prompt_multi_selection(prompt, key, question)?
                    } else {
                        prompt_selection(prompt, key, question)?
                    }
                } else {
                    prompt_string(prompt, key, engine, question.default, current_context)?
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
