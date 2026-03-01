//! Dialoguer-based implementations of prompt interfaces
//!
//! This module provides concrete implementations of the prompt interfaces
//! using the dialoguer library for terminal user interaction.

use super::interface::{
    ConfirmationConfig, MultipleChoiceConfig, SecretConfig, SingleChoiceConfig,
    StructuredDataConfig, TextPromptConfig,
};
use super::theme::PromptTheme;
use crate::{error::Result, prompt::parser::DataParser};
use dialoguer::console::{style, Style};
use dialoguer::{
    theme::ColorfulTheme, Confirm, Editor, Input, MultiSelect, Password, Select,
};
use serde_json::Value;

/// Default terminal-backed prompt provider implemented with `dialoguer`.
pub struct DialoguerPrompter {
    theme: ColorfulTheme,
}

impl DialoguerPrompter {
    pub fn new() -> Self {
        Self::with_theme(PromptTheme::default())
    }

    pub fn with_theme(theme: PromptTheme) -> Self {
        let theme = match theme {
            PromptTheme::Classic => ColorfulTheme::default(),
            PromptTheme::Fancy => Self::baker_theme(),
        };
        Self { theme }
    }

    fn baker_theme() -> ColorfulTheme {
        ColorfulTheme {
            defaults_style: Style::new().for_stderr().cyan(),
            prompt_style: Style::new().for_stderr().bold().cyan(),
            prompt_prefix: style("baker".to_string()).for_stderr().cyan(),
            prompt_suffix: style(">".to_string()).for_stderr().black().bright(),
            success_prefix: style("ok".to_string()).for_stderr().green(),
            success_suffix: style(":".to_string()).for_stderr().black().bright(),
            error_prefix: style("err".to_string()).for_stderr().red(),
            values_style: Style::new().for_stderr().green(),
            active_item_style: Style::new().for_stderr().bold().green(),
            active_item_prefix: style(">".to_string()).for_stderr().green(),
            inactive_item_prefix: style(" ".to_string()).for_stderr(),
            checked_item_prefix: style("[x]".to_string()).for_stderr().green(),
            unchecked_item_prefix: style("[ ]".to_string()).for_stderr().yellow(),
            picked_item_prefix: style(">".to_string()).for_stderr().green(),
            unpicked_item_prefix: style(" ".to_string()).for_stderr(),
            ..ColorfulTheme::default()
        }
    }
}

impl Default for DialoguerPrompter {
    fn default() -> Self {
        Self::new()
    }
}

impl super::interface::TextPrompter for DialoguerPrompter {
    fn prompt_text(&self, config: &TextPromptConfig) -> Result<String> {
        if let Some(secret_config) = &config.secret {
            self.prompt_password(&config.prompt, secret_config)
        } else {
            self.prompt_regular_text(
                &config.prompt,
                config.default.as_deref().unwrap_or(""),
            )
        }
    }
}

impl super::interface::SingleChoicePrompter for DialoguerPrompter {
    fn prompt_single_choice(&self, config: &SingleChoiceConfig) -> Result<usize> {
        let mut select = Select::with_theme(&self.theme)
            .with_prompt(&config.prompt)
            .items(&config.choices);

        if let Some(default_index) = config.default_index {
            select = select.default(default_index);
        }

        Ok(select.interact()?)
    }
}

impl super::interface::MultipleChoicePrompter for DialoguerPrompter {
    fn prompt_multiple_choice(
        &self,
        config: &MultipleChoiceConfig,
    ) -> Result<Vec<usize>> {
        let indices = MultiSelect::with_theme(&self.theme)
            .with_prompt(&config.prompt)
            .items(&config.choices)
            .defaults(&config.defaults)
            .interact()?;

        Ok(indices)
    }
}

impl super::interface::ConfirmationPrompter for DialoguerPrompter {
    fn prompt_confirmation(&self, config: &ConfirmationConfig) -> Result<bool> {
        let result = Confirm::with_theme(&self.theme)
            .with_prompt(&config.prompt)
            .default(config.default)
            .interact()?;

        Ok(result)
    }
}

impl super::interface::StructuredDataPrompter for DialoguerPrompter {
    fn prompt_structured_data(&self, config: &StructuredDataConfig) -> Result<Value> {
        let default_content =
            DataParser::serialize_structured_data(&config.default_value, config.is_yaml)?;

        let options = vec!["Enter in terminal", "Open editor"];

        let selection = Select::with_theme(&self.theme)
            .with_prompt(&config.prompt)
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => self.prompt_terminal_input(&default_content, config.is_yaml),
            1 => self.prompt_editor_input(
                &default_content,
                &config.file_extension,
                config.is_yaml,
            ),
            _ => unreachable!(),
        }
    }
}

impl DialoguerPrompter {
    /// Handle password input with optional confirmation
    fn prompt_password(
        &self,
        prompt: &str,
        secret_config: &SecretConfig,
    ) -> Result<String> {
        let mut password = Password::with_theme(&self.theme).with_prompt(prompt);

        if secret_config.confirm {
            let error_message = if secret_config.mismatch_error.is_empty() {
                "Mismatch".to_string()
            } else {
                secret_config.mismatch_error.clone()
            };

            password =
                password.with_confirmation(format!("{prompt} (confirm)"), error_message);
        }

        Ok(password.interact()?)
    }

    /// Handle regular text input
    fn prompt_regular_text(&self, prompt: &str, default: &str) -> Result<String> {
        Ok(Input::with_theme(&self.theme)
            .with_prompt(prompt)
            .default(default.to_string())
            .interact_text()?)
    }

    /// Handle terminal input for structured data
    fn prompt_terminal_input(
        &self,
        default_content: &str,
        is_yaml: bool,
    ) -> Result<Value> {
        let content: String = Input::with_theme(&self.theme)
            .with_prompt("Enter content")
            .default(default_content.to_string())
            .interact_text()?;

        DataParser::parse_structured_content(&content, is_yaml)
    }

    /// Handle editor input for structured data
    fn prompt_editor_input(
        &self,
        default_content: &str,
        file_extension: &str,
        is_yaml: bool,
    ) -> Result<Value> {
        let content = Editor::new()
            .extension(file_extension)
            .edit(default_content)?
            .unwrap_or_else(|| default_content.to_string());

        DataParser::parse_structured_content(&content, is_yaml)
    }
}
