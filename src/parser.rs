use crate::config::{Config, Question, ValueType};
use crate::error::{BakerError, BakerResult};
use crate::prompt::prompt_answer;
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
pub fn get_default_answers<S: Into<String>>(
    context: S,
) -> BakerResult<serde_json::Value> {
    let context: String = context.into();
    if context.is_empty() {
        return Ok(serde_json::Value::Null);
    }

    serde_json::from_str(&context).map_err(|e| {
        BakerError::TemplateError(format!("Failed to parse context as JSON: {}", e))
    })
}

/// Retrieves the default value of single choice
pub fn get_single_choice_default(questions: &Question) -> serde_json::Value {
    let default_value = if let Some(default_value) = &questions.default {
        if let Some(default_str) = default_value.as_str() {
            questions.choices.iter().position(|choice| choice == default_str).unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    };

    serde_json::Value::Number(default_value.into())
}

pub fn get_text_default(
    question: &Question,
    current_context: &serde_json::Value,
    engine: &dyn TemplateEngine,
) -> serde_json::Value {
    let default_value = if let Some(default_value) = &question.default {
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

pub fn get_yes_no_default(question: &Question) -> serde_json::Value {
    let default_value = if let Some(default_value) = &question.default {
        default_value.as_bool().unwrap_or(false)
    } else {
        false
    };

    serde_json::Value::Bool(default_value)
}

pub fn get_answers(
    engine: &dyn TemplateEngine,
    config: Config,
    default_answers: serde_json::Value,
) -> BakerResult<serde_json::Value> {
    let mut answers = serde_json::Map::new();

    for (key, question) in config.questions {
        let current_context = serde_json::Value::Object(answers.clone());

        let default_answer = default_answers.get(&key);

        let (question_type, default_value) = match question.value_type {
            ValueType::Str => {
                if !question.choices.is_empty() {
                    if question.multiselect {
                        (QuestionType::MultipleChoice, serde_json::Value::Null)
                    } else {
                        // Extracts the default value from config.default (baker.yaml)
                        // if the value contains the template string it renders it.
                        let default_value = get_single_choice_default(&question);
                        (QuestionType::SingleChoice, default_value)
                    }
                } else {
                    let default_value =
                        get_text_default(&question, &current_context, &*engine);
                    (QuestionType::Text, default_value)
                }
            }
            ValueType::Bool => {
                let default_value = get_yes_no_default(&question);
                (QuestionType::YesNo, default_value)
            }
        };

        let (key, value) = if let Some(default_value) = default_answer {
            // Return the default answer
            (key, default_value.clone())
        } else {
            // Sometimes "help" contain the value with the template strings.
            // This function renders it and returns rendered value.
            let help_rendered = engine
                .render(&question.help, &current_context)
                .unwrap_or(question.help.clone());
            prompt_answer(key, question_type, default_value, help_rendered, question)?
        };
        answers.insert(key, value);
    }

    Ok(serde_json::Value::Object(answers))
}
