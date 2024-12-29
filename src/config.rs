//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.

use crate::{
    error::{Error, Result},
    renderer::TemplateRenderer,
};
use indexmap::IndexMap;
use log::debug;
use serde::Deserialize;
use std::path::Path;

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

/// Supported configuration file names
pub const CONFIG_FILES: [&str; 3] = ["baker.json", "baker.yml", "baker.yaml"];

/// Main configuration structure holding all questions
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Map of question identifiers to their configurations
    #[serde(flatten)]
    pub questions: IndexMap<String, Question>,
}

impl Config {
    pub fn new() -> ConfigBuilder {
        ConfigBuilder::new()
    }
}

pub struct ConfigBuilder {
    config: Option<Config>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        ConfigBuilder { config: None }
    }

    pub fn from_yaml<P: AsRef<Path>>(mut self, path: P) -> Self {
        if self.config.is_some() {
            return self;
        }

        if let Ok(contents) = std::fs::read_to_string(path) {
            if let Ok(config) = serde_yaml::from_str(&contents) {
                self.config = Some(config);
            }
        }
        self
    }

    pub fn from_yml<P: AsRef<Path>>(self, path: P) -> Self {
        self.from_yaml(path)
    }

    pub fn from_json<P: AsRef<Path>>(mut self, path: P) -> Self {
        if self.config.is_some() {
            return self;
        }

        if let Ok(contents) = std::fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str(&contents) {
                self.config = Some(config);
            }
        }
        self
    }

    pub fn build(self) -> Result<Config> {
        self.config.ok_or_else(|| Error::BakerIgnoreError("".to_string()))
    }
}

#[derive(Debug)]
pub struct QuestionRendered {
    pub ask_if: bool,
    pub default: serde_json::Value,
    pub help: Option<String>,
    pub r#type: QuestionType,
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
        let default_value: Box<dyn DefaultValue> = match self.into_question_type() {
            QuestionType::SingleChoice => Box::new(SingleChoice),
            QuestionType::MultipleChoice => Box::new(MultipleChoice),
            QuestionType::Text => Box::new(Text),
            QuestionType::Boolean => Box::new(Boolean),
        };

        default_value.get_default(self, answers, engine)
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

        QuestionRendered { default, ask_if, help, r#type: self.into_question_type() }
    }
}

pub struct SingleChoice;
pub struct MultipleChoice;
pub struct Text;
pub struct Boolean;

#[derive(Debug, PartialEq)]
pub enum QuestionType {
    MultipleChoice,
    SingleChoice,
    Text,
    Boolean,
}

/// Default value handler for different question types
pub trait DefaultValue {
    fn get_default(
        &self,
        question: &Question,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
    ) -> serde_json::Value;
}

impl DefaultValue for SingleChoice {
    fn get_default(
        &self,
        question: &Question,
        _answers: &serde_json::Value,
        _engine: &dyn TemplateRenderer,
    ) -> serde_json::Value {
        let default_value = if let Some(default_value) = &question.default {
            if let Some(default_str) = default_value.as_str() {
                question
                    .choices
                    .iter()
                    .position(|choice| choice == default_str)
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        serde_json::Value::Number(default_value.into())
    }
}

impl DefaultValue for MultipleChoice {
    fn get_default(
        &self,
        question: &Question,
        _answers: &serde_json::Value,
        _engine: &dyn TemplateRenderer,
    ) -> serde_json::Value {
        let default_value = question
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

        let defaults_map: Vec<bool> = question
            .choices
            .iter()
            .map(|choice| default_value.contains_key(choice))
            .collect();

        serde_json::to_value(defaults_map).unwrap()
    }
}

impl DefaultValue for Text {
    fn get_default(
        &self,
        question: &Question,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
    ) -> serde_json::Value {
        let default_value = if let Some(default_value) = &question.default {
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
}

impl DefaultValue for Boolean {
    fn get_default(
        &self,
        question: &Question,
        _answers: &serde_json::Value,
        _engine: &dyn TemplateRenderer,
    ) -> serde_json::Value {
        let default_value = if let Some(default_value) = &question.default {
            default_value.as_bool().unwrap_or(false)
        } else {
            false
        };

        serde_json::Value::Bool(default_value)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::renderer::MiniJinjaRenderer;

    use super::*;

    #[test]
    fn it_works_1() {
        let question = Question {
            help: "Hello, {{prev_answer}}".to_string(),
            r#type: Type::Bool,
            default: None,
            ask_if: r#"prev_answer == "TEST""#.to_string(),
            secret: None,
            multiselect: false,
            choices: vec![],
        };
        let engine = Box::new(MiniJinjaRenderer::new());

        let answers = json!({
            "prev_answer": "World"
        });

        let result = question.render(&answers, &*engine);
        match result {
            QuestionRendered { ask_if, help, default, r#type } => {
                assert_eq!(ask_if, false);
                assert_eq!(help, Some("Hello, World".to_string()));
                assert_eq!(default, serde_json::Value::Bool(false));
                assert_eq!(r#type, QuestionType::Boolean);
            }
        }
    }

    #[test]
    fn it_works_2() {
        let question = Question {
            help: "{{question}}".to_string(),
            r#type: Type::Str,
            default: Some(json!(vec!["Python".to_string(), "Django".to_string()])),
            ask_if: "".to_string(),
            secret: None,
            multiselect: true,
            choices: vec![
                "Python".to_string(),
                "Django".to_string(),
                "FastAPI".to_string(),
                "Next.JS".to_string(),
                "TypeScript".to_string(),
            ],
        };
        let engine = Box::new(MiniJinjaRenderer::new());

        let answers = json!({
            "question": "Please select your stack"
        });

        let result = question.render(&answers, &*engine);
        match result {
            QuestionRendered { ask_if, help, default, r#type } => {
                assert_eq!(ask_if, true);
                assert_eq!(help, Some("Please select your stack".to_string()));
                assert_eq!(default, json!(vec![true, true, false, false, false]));
                assert_eq!(r#type, QuestionType::MultipleChoice);
            }
        }
    }
}
