use crate::config::{ConfigItem, ConfigItemType};
use crate::error::{BakerError, BakerResult};
use crate::prompt::{
    prompt_boolean, prompt_multiple_choice, prompt_single_choice, prompt_string,
};
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

/// Returns the question value and its key
pub fn get_question_value(
    help_rendered: String,
    key: String,
    config_item: ConfigItem,
    default_value: serde_json::Value,
    question_type: QuestionType,
    parsed: &serde_json::Value,
) -> BakerResult<(String, serde_json::Value)> {
    if parsed.is_null() {
        match question_type {
            QuestionType::MultipleChoice => {
                // when
                // type: str
                // choices: ...
                // multiselect: true
                prompt_multiple_choice(help_rendered, key, config_item)
            }
            QuestionType::SingleChoice => {
                // when
                // type: str
                // choices: ...
                // multiselect: false
                prompt_single_choice(help_rendered, key, config_item, default_value)
            }
            QuestionType::YesNo => {
                // when
                // type: bool
                prompt_boolean(help_rendered, key, default_value)
            }
            QuestionType::Text => {
                // when
                // type: str
                prompt_string(help_rendered, key, config_item, default_value)
            }
        }
    } else {
        get_value_or_default(key, parsed.clone(), default_value)
    }
}
