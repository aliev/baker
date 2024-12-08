//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.

use crate::config::ConfigItem;
use crate::error::{BakerError, BakerResult};

use dialoguer::{Confirm, Input, MultiSelect, Password, Select};

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
pub fn prompt_multiple_choice(
    prompt: String,
    key: String,
    question: ConfigItem,
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
pub fn prompt_single_choice(
    prompt: String,
    key: String,
    question: ConfigItem,
    default_value: serde_json::Value,
) -> BakerResult<(String, serde_json::Value)> {
    let default_value: usize = default_value.as_u64().unwrap() as usize;
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
pub fn prompt_string(
    prompt: String,
    key: String,
    question: ConfigItem,
    default_value: serde_json::Value,
) -> BakerResult<(String, serde_json::Value)> {
    let default_str = match default_value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Null => String::new(),
        _ => default_value.to_string(),
    };

    let input = if question.secret {
        let mut password = Password::new().with_prompt(&prompt);

        if question.secret_confirmation {
            password =
                password.with_confirmation(format!("{} (confirm)", &prompt), "Mistmatch");
        }

        password.interact().map_err(|e| {
            BakerError::ConfigError(format!("failed to get user input: {}", e))
        })?
    } else {
        Input::new().with_prompt(&prompt).default(default_str).interact_text().map_err(
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
///
/// # Returns
/// * `BakerResult<(String, serde_json::Value)>` - The key and boolean value
///
/// # Errors
/// * Returns `BakerError::ConfigError` if user interaction fails
pub fn prompt_boolean(
    prompt: String,
    key: String,
    default_value: serde_json::Value,
) -> BakerResult<(String, serde_json::Value)> {
    let default_value = default_value.as_bool().unwrap();
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
