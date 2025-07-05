//! Question configuration and rendering logic

use crate::config::types::{
    get_default_validation, QuestionType, Secret, Type, Validation,
};
use crate::renderer::TemplateRenderer;
use serde::Deserialize;

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
    pub default: serde_json::Value,
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
    /// JSON Schema for validation (for Json and Yaml types)
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default = "get_default_validation")]
    pub validation: Validation,
}

#[derive(Debug)]
pub struct QuestionRendered {
    pub ask_if: bool,
    pub default: serde_json::Value,
    pub help: String,
    pub r#type: QuestionType,
}

pub trait IntoQuestionType {
    #[allow(clippy::wrong_self_convention)]
    fn into_question_type(&self) -> QuestionType;
}

impl IntoQuestionType for Question {
    fn into_question_type(&self) -> QuestionType {
        match (&self.r#type, self.choices.is_empty()) {
            (Type::Str, false) => {
                if self.multiselect {
                    QuestionType::MultipleChoice
                } else {
                    QuestionType::SingleChoice
                }
            }
            (Type::Str, true) => QuestionType::Text,
            (Type::Bool, _) => QuestionType::Boolean,
            (Type::Json, _) => QuestionType::Json,
            (Type::Yaml, _) => QuestionType::Yaml,
        }
    }
}

impl Question {
    fn process_structured_default_value(
        &self,
        default: serde_json::Value,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
        question_type: &QuestionType,
    ) -> serde_json::Value {
        // If the default is already a JSON object or array, use it directly
        if default.is_object() || default.is_array() {
            default
        } else if let Some(default_str) = default.as_str() {
            // If it's a string, try to render it as a template first
            let rendered_str = engine
                .render(default_str, answers, Some("default_value"))
                .unwrap_or(default_str.to_string());

            // Parse the string based on the question type
            match question_type {
                QuestionType::Json => {
                    serde_json::from_str(&rendered_str).unwrap_or(serde_json::json!({}))
                }
                QuestionType::Yaml => {
                    serde_yaml::from_str(&rendered_str).unwrap_or(serde_json::json!({}))
                }
                _ => unreachable!(),
            }
        } else {
            // Fallback to empty object
            serde_json::json!({})
        }
    }

    pub fn render(
        &self,
        question_key: &str,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
    ) -> QuestionRendered {
        // Renders default.
        let default = if let Some(answer) = answers.get(question_key) {
            // If answer in pre-filled answers we just return them as it is.
            answer.to_owned()
        } else {
            let default = self.default.clone();
            match self.into_question_type() {
                QuestionType::MultipleChoice => default,
                QuestionType::Boolean => {
                    let val = default.as_bool().unwrap_or(false);
                    serde_json::Value::Bool(val)
                }
                QuestionType::SingleChoice | QuestionType::Text => {
                    // Trying to extract str from default which is serde_json::Value,
                    // otherwise it return empty slice.
                    let default_str = default.as_str().unwrap_or_default();

                    // Trying to render given string.
                    // Otherwise returns an empty string.
                    let default_rendered = engine
                        .render(default_str, answers, Some("default_value"))
                        .unwrap_or_default();
                    serde_json::Value::String(default_rendered)
                }
                QuestionType::Json | QuestionType::Yaml => self
                    .process_structured_default_value(
                        default,
                        answers,
                        engine,
                        &self.into_question_type(),
                    ),
            }
        };

        // Sometimes "help" contain the value with the template strings.
        // This function renders it and returns rendered value.
        let help =
            engine.render(&self.help, answers, Some("help")).unwrap_or(self.help.clone());

        let ask_if = engine.execute_expression(&self.ask_if, answers).unwrap_or(true);

        QuestionRendered { default, ask_if, help, r#type: self.into_question_type() }
    }
}
