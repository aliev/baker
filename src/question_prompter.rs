use crate::{
    error::{Error, Result},
    question::{IntoQuestionType, Question, QuestionType},
};
use dialoguer::{Confirm, Input, MultiSelect, Password, Select};

pub trait QuestionPrompter {
    fn ask(
        &self,
        default_value: serde_json::Value,
        prompt: String,
    ) -> Result<serde_json::Value>;
}

trait PromptHandler {
    fn handle_prompt(
        &self,
        question: &Question,
        default_value: serde_json::Value,
        prompt: String,
    ) -> Result<serde_json::Value>;
}

struct MultipleChoicePrompt;
struct SingleChoicePrompt;
struct TextPrompt;
struct BooleanPrompt;

impl PromptHandler for MultipleChoicePrompt {
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

impl PromptHandler for SingleChoicePrompt {
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

impl PromptHandler for TextPrompt {
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

impl PromptHandler for BooleanPrompt {
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

impl QuestionPrompter for Question {
    fn ask(
        &self,
        default_value: serde_json::Value,
        prompt: String,
    ) -> Result<serde_json::Value> {
        let handler: Box<dyn PromptHandler> = match self.into_question_type() {
            QuestionType::MultipleChoice => Box::new(MultipleChoicePrompt),
            QuestionType::SingleChoice => Box::new(SingleChoicePrompt),
            QuestionType::Text => Box::new(TextPrompt),
            QuestionType::Boolean => Box::new(BooleanPrompt),
        };

        handler.handle_prompt(self, default_value, prompt)
    }
}
