//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.
use serde::Deserialize;

use crate::renderer::TemplateRenderer;
/// Type of question to be presented to the user
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    /// String input question type
    Str,
    /// Boolean (yes/no) question type
    Bool,
}
#[derive(Debug, Deserialize)]
pub struct Secret {
    /// Whether the secret should have confirmation
    #[serde(default)]
    pub confirm: bool,
    #[serde(default)]
    pub mistmatch_err: String,
}

/// Represents a single question in the configuration
#[derive(Debug, Deserialize)]
pub struct Question {
    /// Help text/prompt to display to the user
    #[serde(default)]
    pub help: String,
    /// Type of the question (string or boolean)
    #[serde(rename = "type")]
    pub r#type: Type,
    /// Optional default value for the question
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    /// Available choices for string questions
    #[serde(default)]
    pub choices: Vec<String>,
    /// Available option for string questions
    #[serde(default)]
    pub multiselect: bool,
    /// Whether the string is a secret
    #[serde(default)]
    pub secret: Option<Secret>,
    #[serde(default)]
    pub ask_if: String,
}

pub struct RenderedQuestion {
    pub ask_if: bool,
    pub default: serde_json::Value,
    pub help: Option<String>,
}

pub enum QuestionType {
    MultipleChoice,
    SingleChoice,
    Text,
    Boolean,
}

pub struct QuestionRendered {
    pub ask_if: bool,
    pub default: serde_json::Value,
    pub help: Option<String>,
}

pub trait IntoQuestionType {
    fn into_question_type(&self) -> QuestionType;
}

impl IntoQuestionType for Question {
    fn into_question_type(&self) -> QuestionType {
        match (&self.r#type, self.choices.is_empty(), self.multiselect) {
            (Type::Str, false, true) => QuestionType::MultipleChoice,
            (Type::Str, false, false) => QuestionType::SingleChoice,
            (Type::Str, true, _) => QuestionType::Text,
            (Type::Bool, _, _) => QuestionType::Boolean,
        }
    }
}

impl<'a> Question {
    fn get_default(
        &self,
        answers: &serde_json::Value,
        engine: &'a dyn TemplateRenderer,
    ) -> serde_json::Value {
        match self.into_question_type() {
            QuestionType::MultipleChoice => self.get_multiple_choice_default(),
            QuestionType::SingleChoice => self.get_single_choice_default(),
            QuestionType::Text => self.get_text_default(answers, engine),
            QuestionType::Boolean => self.get_yes_no_default(),
        }
    }

    pub fn render(
        &self,
        answers: &serde_json::Value,
        engine: &'a dyn TemplateRenderer,
    ) -> QuestionRendered {
        let default = self.get_default(answers, engine);

        // Sometimes "help" contain the value with the template strings.
        // This function renders it and returns rendered value.
        let help = Some(engine.render(&self.help, answers).unwrap_or(self.help.clone()));

        let ask_if = engine.execute_expression(&self.ask_if, answers).unwrap_or(true);

        QuestionRendered { default, ask_if, help }
    }

    /// Retrieves the default value of single choice
    fn get_single_choice_default(&self) -> serde_json::Value {
        let default_value = if let Some(default_value) = &self.default {
            if let Some(default_str) = default_value.as_str() {
                self.choices.iter().position(|choice| choice == default_str).unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        serde_json::Value::Number(default_value.into())
    }

    fn get_multiple_choice_default(&self) -> serde_json::Value {
        let default_value = self
            .default
            .as_ref()
            .and_then(|default_value| {
                if let Some(default_obj) = default_value.as_object() {
                    Some(default_obj.clone())
                } else if let Some(default_arr) = default_value.as_array() {
                    let map = default_arr
                        .iter()
                        .filter_map(|value| {
                            value
                                .as_str()
                                .map(|s| (s.to_string(), serde_json::Value::Bool(true)))
                        })
                        .collect();
                    Some(map)
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let defaults_map: Vec<bool> = self
            .choices
            .iter()
            .map(|choice| default_value.contains_key(choice))
            .collect();

        serde_json::to_value(defaults_map).unwrap()
    }

    fn get_text_default(
        &self,
        answers: &serde_json::Value,
        engine: &'a dyn TemplateRenderer,
    ) -> serde_json::Value {
        let default_value = if let Some(default_value) = &self.default {
            if let Some(s) = default_value.as_str() {
                engine.render(s, answers).unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        serde_json::Value::String(default_value)
    }

    fn get_yes_no_default(&self) -> serde_json::Value {
        let default_value = if let Some(default_value) = &self.default {
            default_value.as_bool().unwrap_or(false)
        } else {
            false
        };

        serde_json::Value::Bool(default_value)
    }
}
