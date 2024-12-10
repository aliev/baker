use crate::config::{Config, ConfigItem, ConfigItemType};
use crate::error::{BakerError, BakerResult};
use crate::prompt::{
    prompt_boolean, prompt_multiple_choice, prompt_single_choice, prompt_string,
};
use crate::template::TemplateEngine;

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

/// Retrieves the default value of single choice
pub fn get_single_choice_default(item: &ConfigItem) -> serde_json::Value {
    let default_value = if let Some(default_value) = &item.default {
        if let Some(default_str) = default_value.as_str() {
            item.choices.iter().position(|choice| choice == default_str).unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    };

    serde_json::Value::Number(default_value.into())
}

pub fn get_text_default(
    item: &ConfigItem,
    current_context: serde_json::Value,
    engine: &dyn TemplateEngine,
) -> serde_json::Value {
    let default_value = if let Some(default_value) = &item.default {
        if let Some(s) = default_value.as_str() {
            engine.render(s, &current_context).unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    serde_json::Value::String(default_value)
}

pub fn get_yes_no_default(item: &ConfigItem) -> serde_json::Value {
    let default_value = if let Some(default_value) = &item.default {
        default_value.as_bool().unwrap_or(false)
    } else {
        false
    };

    serde_json::Value::Bool(default_value)
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

pub fn get_context(
    engine: &dyn TemplateEngine,
    config: Config,
    parsed: serde_json::Value,
) -> BakerResult<serde_json::Value> {
    let mut answers = serde_json::Map::new();

    for (key, item) in config.items {
        let current_context = serde_json::Value::Object(answers.clone());

        // Sometimes "help" contain the value with the template strings.
        // This function renders it and returns rendered value.
        let help_rendered =
            engine.render(&item.help, &current_context).unwrap_or(item.help.clone());

        match item.item_type {
            ConfigItemType::Str => {
                let (key, value) = if !item.choices.is_empty() {
                    if item.multiselect {
                        get_question_value(
                            help_rendered,
                            key,
                            item,
                            serde_json::Value::Null,
                            QuestionType::MultipleChoice,
                            &parsed,
                        )?
                    } else {
                        // Extracts the default value from config.default (baker.yaml)
                        // if the value contains the template string it renders it.
                        let default_value = get_single_choice_default(&item);

                        // This function decides from where to get the value
                        // from user's input or from the defaults.
                        get_question_value(
                            help_rendered,
                            key,
                            item,
                            default_value,
                            QuestionType::SingleChoice,
                            &parsed,
                        )?
                    }
                } else {
                    let default_value =
                        get_text_default(&item, current_context, &*engine);
                    get_question_value(
                        help_rendered,
                        key,
                        item,
                        default_value,
                        QuestionType::Text,
                        &parsed,
                    )?
                };
                answers.insert(key, value);
            }
            ConfigItemType::Bool => {
                let default_value = get_yes_no_default(&item);
                let (key, value) = get_question_value(
                    help_rendered,
                    key,
                    item,
                    default_value,
                    QuestionType::YesNo,
                    &parsed,
                )?;

                answers.insert(key, value);
            }
        };
    }

    let context = serde_json::Value::Object(answers);
    Ok(context)
}
