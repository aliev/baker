//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.
use serde::Deserialize;
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

pub trait WithQuestionType {
    fn question_type(&self) -> QuestionType;
}

impl WithQuestionType for Question {
    fn question_type(&self) -> QuestionType {
        match (&self.r#type, self.choices.is_empty(), self.multiselect) {
            (Type::Str, false, true) => QuestionType::MultipleChoice,
            (Type::Str, false, false) => QuestionType::SingleChoice,
            (Type::Str, true, _) => QuestionType::Text,
            (Type::Bool, _, _) => QuestionType::Boolean,
        }
    }
}
