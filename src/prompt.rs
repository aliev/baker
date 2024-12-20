//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.

use crate::config::Question;
use crate::error::{Error, Result};
use crate::parser::QuestionType;

use dialoguer::{Confirm, Input, MultiSelect, Password, Select};

pub trait Prompter {
    fn multiple_choice(
        &self,
        prompt: String,
        question: Question,
        default_value: serde_json::Value,
    ) -> Result<serde_json::Value>;

    fn single_choice(
        &self,
        prompt: String,
        question: Question,
        default_value: serde_json::Value,
    ) -> Result<serde_json::Value>;

    fn string(
        &self,
        prompt: String,
        question: Question,
        default_value: serde_json::Value,
    ) -> Result<serde_json::Value>;

    fn boolean(
        &self,
        prompt: String,
        default_value: serde_json::Value,
    ) -> Result<serde_json::Value>;

    fn confirm(&self, skip: bool, prompt: String) -> Result<bool>;

    fn answer(
        &self,
        question_type: QuestionType,
        default_value: serde_json::Value,
        prompt: String,
        question: Question,
    ) -> Result<serde_json::Value> {
        match question_type {
            QuestionType::MultipleChoice => {
                self.multiple_choice(prompt, question, default_value)
            }
            QuestionType::SingleChoice => {
                self.single_choice(prompt, question, default_value)
            }
            QuestionType::YesNo => self.boolean(prompt, default_value),
            QuestionType::Text => self.string(prompt, question, default_value),
        }
    }
}

pub struct DialoguerPrompter;

impl DialoguerPrompter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DialoguerPrompter {
    fn default() -> Self {
        Self
    }
}

impl Prompter for DialoguerPrompter {
    fn multiple_choice(
        &self,
        prompt: String,
        question: Question,
        default_value: serde_json::Value,
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

    fn single_choice(
        &self,
        prompt: String,
        question: Question,
        default_value: serde_json::Value,
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

    fn string(
        &self,
        prompt: String,
        question: Question,
        default_value: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let default_str = match default_value {
            serde_json::Value::String(s) => s,
            serde_json::Value::Null => String::new(),
            _ => default_value.to_string(),
        };

        let input = if question.secret {
            let mut password = Password::new().with_prompt(&prompt);

            if question.secret_confirmation {
                password = password
                    .with_confirmation(format!("{} (confirm)", &prompt), "Mismatch");
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

    fn boolean(
        &self,
        prompt: String,
        default_value: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let default_value = default_value.as_bool().unwrap();
        let result = Confirm::new()
            .with_prompt(prompt)
            .default(default_value)
            .interact()
            .map_err(Error::PromptError)?;

        Ok(serde_json::Value::Bool(result))
    }

    fn confirm(&self, skip: bool, prompt: String) -> Result<bool> {
        if skip {
            return Ok(true);
        }
        Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()
            .map_err(Error::PromptError)
    }
}
