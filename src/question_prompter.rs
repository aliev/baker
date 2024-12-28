use crate::{
    error::{Error, Result},
    question::{IntoQuestionType, Question, QuestionType},
};
use dialoguer::{Confirm, Input, MultiSelect, Password, Select};

pub trait QuestionPrompter: IntoQuestionType {
    fn ask(
        &self,
        default_value: serde_json::Value,
        prompt: String,
    ) -> Result<serde_json::Value> {
        match self.into_question_type() {
            QuestionType::MultipleChoice => self.multiple_choice(prompt, default_value),
            QuestionType::SingleChoice => self.single_choice(prompt, default_value),
            QuestionType::Text => self.string(prompt, default_value),
            QuestionType::Boolean => self.boolean(prompt, default_value),
        }
    }
    fn multiple_choice(
        &self,
        prompt: String,
        default_value: serde_json::Value,
    ) -> Result<serde_json::Value>;

    fn single_choice(
        &self,
        prompt: String,
        default_value: serde_json::Value,
    ) -> Result<serde_json::Value>;

    fn string(
        &self,
        prompt: String,
        default_value: serde_json::Value,
    ) -> Result<serde_json::Value>;

    fn boolean(
        &self,
        prompt: String,
        default_value: serde_json::Value,
    ) -> Result<serde_json::Value>;
}

impl QuestionPrompter for Question {
    fn multiple_choice(
        &self,
        prompt: String,
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
            .items(&self.choices)
            .defaults(&defaults)
            .interact()
            .map_err(Error::PromptError)?;

        let selected: Vec<serde_json::Value> = indices
            .iter()
            .map(|&i| serde_json::Value::String(self.choices[i].clone()))
            .collect();

        Ok(serde_json::Value::Array(selected))
    }

    fn single_choice(
        &self,
        prompt: String,
        default_value: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let default_value: usize = default_value.as_u64().unwrap() as usize;
        let selection = Select::new()
            .with_prompt(prompt)
            .default(default_value)
            .items(&self.choices)
            .interact()
            .map_err(Error::PromptError)?;

        Ok(serde_json::Value::String(self.choices[selection].clone()))
    }

    fn string(
        &self,
        prompt: String,
        default_value: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let default_str = match default_value {
            serde_json::Value::String(s) => s,
            serde_json::Value::Null => String::new(),
            _ => default_value.to_string(),
        };

        let input = if let Some(secret) = &self.secret {
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
}
