//! Basic types and enums for configuration

use crate::constants::validation;
use serde::Deserialize;

/// Type of question to be presented to the user
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    /// String input question type
    Str,
    /// Boolean (yes/no) question type
    Bool,
    /// JSON structured input type
    Json,
    /// YAML structured input type
    Yaml,
}

#[derive(Debug, Deserialize)]
pub struct Secret {
    /// Whether the secret should have confirmation
    #[serde(default)]
    pub confirm: bool,
    #[serde(default = "get_default_mismatch_error")]
    pub mistmatch_err: String,
}

#[derive(Debug, Deserialize)]
pub struct Validation {
    #[serde(default = "get_default_condition")]
    pub condition: String,
    #[serde(default = "get_default_error_message")]
    pub error_message: String,
}

#[derive(Debug, PartialEq)]
pub enum QuestionType {
    MultipleChoice,
    SingleChoice,
    Text,
    Boolean,
    Json,
    Yaml,
}

fn get_default_error_message() -> String {
    validation::INVALID_ANSWER.to_string()
}

fn get_default_mismatch_error() -> String {
    validation::PASSWORDS_MISMATCH.to_string()
}

pub fn get_default_condition() -> String {
    validation::DEFAULT_CONDITION.to_string()
}

pub fn get_default_validation() -> Validation {
    Validation {
        condition: get_default_condition(),
        error_message: get_default_error_message(),
    }
}
