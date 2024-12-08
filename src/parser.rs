use crate::config::{ConfigItem, ConfigItemType};
use crate::error::BakerResult;
use crate::template::TemplateEngine;
use indexmap::IndexMap;

pub enum QuestionType {
    MultipleChoice,
    SingleChoice,
    Text,
    YesNo,
}

pub fn parse_default_context(
    key: String,
    parsed: serde_json::Value,
    default_value: serde_json::Value,
) -> BakerResult<(String, serde_json::Value)> {
    let value = parsed.get(&key);
    let result_value = if value.is_some() {
        value.cloned().unwrap_or(serde_json::Value::Null)
    } else if !default_value.is_null() {
        default_value.clone()
    } else {
        serde_json::Value::Null
    };

    Ok((key, result_value))
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
