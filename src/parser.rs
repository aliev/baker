use crate::config::{ConfigItem, ConfigItemType};
use crate::error::{BakerError, BakerResult};
use crate::template::TemplateEngine;
use indexmap::IndexMap;

pub enum QuestionType {
    MultipleChoice,
    SingleChoice,
    Text,
    YesNo,
}

/// Retrieves a value from the provided `context` using the specified `key`.
/// If the key does not exist in the `context` or its value is `serde_json::Value::Null`,
/// the function falls back to the provided `default` if it is not `serde_json::Value::Null`.
/// If both are `serde_json::Value::Null` or the key does not exist, it returns a `serde_json::Value::Null`.
/// The result is returned as a tuple containing the `key` and the resolved value.
///
/// # Parameters
/// - `key`: A string or type convertible into a `String` representing the JSON key.
/// - `context`: The context value that represents the parsed JSON.
/// - `default`: The default value.
///
/// # Returns
/// - `Ok(serde_json::Value)` if the input string is valid JSON or empty.
/// - `Err(BakerError)` if the input string is not valid JSON.
pub fn get_value_or_default<S: Into<String>>(
    key: S,
    context: serde_json::Value,
    default: serde_json::Value,
) -> BakerResult<(String, serde_json::Value)> {
    let key: String = key.into();
    let result_value = context.get(&key).cloned().unwrap_or_else(|| {
        if !default.is_null() {
            default.clone()
        } else {
            serde_json::Value::Null
        }
    });

    Ok((key, result_value))
}

/// Parses a string representation of a JSON value into a `serde_json::Value`.
/// If the input string is empty, the function returns a `serde_json::Value::Null`.
/// If the input string is invalid JSON, it returns a `BakerError` with details about the parsing failure.
///
/// # Parameters
/// - `context`: A string or type convertible into a `String` representing the JSON to parse.
///
/// # Returns
/// - `Ok(serde_json::Value)` if the input string is valid JSON or empty.
/// - `Err(BakerError)` if the input string is not valid JSON.
pub fn get_context_value<S: Into<String>>(context: S) -> BakerResult<serde_json::Value> {
    let context: String = context.into();
    if context.is_empty() {
        return Ok(serde_json::Value::Null);
    }

    serde_json::from_str(&context).map_err(|e| {
        BakerError::TemplateError(format!("Failed to parse context as JSON: {}", e))
    })
}

/// Parses answers from user's questions
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
pub fn parse_questions(
    config_items: IndexMap<String, ConfigItem>,
    engine: &dyn TemplateEngine,
    callback: impl Fn(
        String,
        String,
        ConfigItem,
        serde_json::Value,
        QuestionType,
    ) -> BakerResult<(String, serde_json::Value)>,
) -> BakerResult<serde_json::Value> {
    let mut answers = serde_json::Map::new();

    for (key, item) in config_items {
        let current_context = serde_json::Value::Object(answers.clone());
        // Sometimes "help" contain the value with the template strings.
        // This function renders it and returns rendered value.
        let help_rendered =
            engine.render(&item.help, &current_context).unwrap_or(item.help.clone());

        match item.item_type {
            ConfigItemType::Str => {
                let (key, value) = if !item.choices.is_empty() {
                    if item.multiselect {
                        callback(
                            help_rendered,
                            key,
                            item,
                            serde_json::Value::Null,
                            QuestionType::MultipleChoice,
                        )?
                    } else {
                        let default_value = if let Some(default_value) = &item.default {
                            if let Some(default_str) = default_value.as_str() {
                                item.choices
                                    .iter()
                                    .position(|choice| choice == default_str)
                                    .unwrap_or(0)
                            } else {
                                0
                            }
                        } else {
                            0
                        };
                        callback(
                            help_rendered,
                            key,
                            item,
                            serde_json::Value::Number(default_value.into()),
                            QuestionType::SingleChoice,
                        )?
                    }
                } else {
                    let default_value = if let Some(default_value) = &item.default {
                        if let Some(s) = default_value.as_str() {
                            engine.render(s, &current_context).unwrap_or_default()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    callback(
                        help_rendered,
                        key,
                        item,
                        serde_json::Value::String(default_value),
                        QuestionType::Text,
                    )?
                };
                answers.insert(key, value);
            }
            ConfigItemType::Bool => {
                let default_value = if let Some(default_value) = &item.default {
                    default_value.as_bool().unwrap_or(false)
                } else {
                    false
                };
                let (key, value) = callback(
                    help_rendered,
                    key,
                    item,
                    serde_json::Value::Bool(default_value),
                    QuestionType::YesNo,
                )?;

                answers.insert(key, value);
            }
        };
    }

    Ok(serde_json::Value::Object(answers))
}
