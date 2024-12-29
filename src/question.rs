//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.
use crate::{
    error::{Error, Result},
    renderer::TemplateRenderer,
};
use dialoguer::{Confirm, Input, MultiSelect, Password, Select};
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

#[derive(Debug, PartialEq)]
pub enum QuestionType {
    MultipleChoice,
    SingleChoice,
    Text,
    Boolean,
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

/// Default value handler for different question types
trait DefaultValue {
    fn get_default(
        &self,
        question: &Question,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
    ) -> serde_json::Value;
}

trait PromptHandler {
    fn handle_prompt(
        &self,
        question: &Question,
        default_value: serde_json::Value,
        prompt: String,
    ) -> Result<serde_json::Value>;
}

struct SingleChoice;
struct MultipleChoice;
struct Text;
struct Boolean;

impl PromptHandler for MultipleChoice {
    fn handle_prompt(
        &self,
        question: &Question,
        default_value: serde_json::Value,
        prompt: String,
    ) -> Result<serde_json::Value> {
        let defaults = default_value
            .as_array()
            .map(|arr| {
                arr.iter().map(|v| v.as_bool().unwrap_or(false)).collect::<Vec<bool>>()
            })
            .unwrap_or_default();

        let indices = MultiSelect::new()
            .with_prompt(prompt)
            .items(&question.choices)
            .defaults(&defaults)
            .interact()
            .map_err(Error::PromptError)?;

        let selected: Vec<serde_json::Value> = indices
            .iter()
            .map(|&i| serde_json::Value::String(question.choices[i].clone()))
            .collect();

        Ok(serde_json::Value::Array(selected))
    }
}

impl PromptHandler for SingleChoice {
    fn handle_prompt(
        &self,
        question: &Question,
        default_value: serde_json::Value,
        prompt: String,
    ) -> Result<serde_json::Value> {
        let default_value: usize = default_value.as_u64().unwrap() as usize;
        let selection = Select::new()
            .with_prompt(prompt)
            .default(default_value)
            .items(&question.choices)
            .interact()
            .map_err(Error::PromptError)?;

        Ok(serde_json::Value::String(question.choices[selection].clone()))
    }
}

impl PromptHandler for Text {
    fn handle_prompt(
        &self,
        question: &Question,
        default_value: serde_json::Value,
        prompt: String,
    ) -> Result<serde_json::Value> {
        let default_str = match default_value {
            serde_json::Value::String(s) => s,
            serde_json::Value::Null => String::new(),
            _ => default_value.to_string(),
        };

        let input = if let Some(secret) = &question.secret {
            let mut password = Password::new().with_prompt(&prompt);

            if secret.confirm {
                password = password.with_confirmation(
                    format!("{} (confirm)", &prompt),
                    if secret.mistmatch_err.is_empty() {
                        "Mistmatch".to_string()
                    } else {
                        secret.mistmatch_err.clone()
                    },
                );
            }

            password.interact().map_err(Error::PromptError)?
        } else {
            Input::new()
                .with_prompt(&prompt)
                .default(default_str)
                .interact_text()
                .map_err(Error::PromptError)?
        };

        Ok(serde_json::Value::String(input))
    }
}

impl PromptHandler for Boolean {
    fn handle_prompt(
        &self,
        _question: &Question,
        default_value: serde_json::Value,
        prompt: String,
    ) -> Result<serde_json::Value> {
        let default_value = default_value.as_bool().unwrap();
        let result = Confirm::new()
            .with_prompt(prompt)
            .default(default_value)
            .interact()
            .map_err(Error::PromptError)?;

        Ok(serde_json::Value::Bool(result))
    }
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

    pub fn ask(
        &self,
        default_value: serde_json::Value,
        prompt: String,
    ) -> Result<serde_json::Value> {
        let prompt_handler: Box<dyn PromptHandler> = match self.into_question_type() {
            QuestionType::MultipleChoice => Box::new(MultipleChoice),
            QuestionType::SingleChoice => Box::new(SingleChoice),
            QuestionType::Text => Box::new(Text),
            QuestionType::Boolean => Box::new(Boolean),
        };

        prompt_handler.handle_prompt(self, default_value, prompt)
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
