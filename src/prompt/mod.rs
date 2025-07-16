//! Interactive dialog utilities for user input
//!
//! This module provides a modular approach to handling different types of user prompts
//! including text input, choices, confirmations, and structured data.
//!
//! The module is structured in layers:
//! - `interface`: Pure abstract interfaces independent of any UI library
//! - `dialoguer`: Concrete implementation using the dialoguer library
//! - `factory`: Factory for creating and executing prompts based on configuration

use crate::{
    config::{Question, Type},
    error::Result,
    prompt::dialoguer::DialoguerPrompter,
};

pub mod dialoguer;
pub mod handler;
pub mod interface;
pub mod parser;

// Re-export new interfaces for easy access
pub use interface::*;

/// Common interface for all prompt types
pub trait Prompter<'a> {
    fn prompt(&self, prompt_context: &PromptContext<'a>) -> Result<serde_json::Value>;
}

/// Context for prompting operations
pub struct PromptContext<'a> {
    pub question: &'a Question,
    pub default: &'a serde_json::Value,
    pub help: &'a str,
}

impl<'a> PromptContext<'a> {
    pub fn new(
        question: &'a Question,
        default: &'a serde_json::Value,
        help: &'a str,
    ) -> Self {
        Self { question, default, help }
    }
}

/// Convenience function to create the default prompt provider
pub fn get_prompt_provider() -> impl PromptProvider {
    DialoguerPrompter::new()
}

/// Main entry point for asking questions
pub fn ask_question(
    question: &Question,
    default: &serde_json::Value,
    help: String,
) -> Result<serde_json::Value> {
    let context = PromptContext::new(question, default, &help);
    let provider = get_prompt_provider();
    let prompt_handler = handler::PromptHandler::new(provider);
    prompt_handler.create_prompt(&context)
}

/// Simple confirmation function for backward compatibility
pub fn confirm(skip: bool, prompt: String) -> Result<bool> {
    if skip {
        return Ok(true);
    }

    let default_validation = crate::config::types::get_default_validation();
    let question = Question {
        help: prompt,
        r#type: Type::Bool,
        default: serde_json::Value::Bool(false),
        choices: Vec::new(),
        multiselect: false,
        secret: None,
        ask_if: String::new(),
        schema: None,
        validation: default_validation,
    };

    let default_value = serde_json::Value::Bool(false);
    let context = PromptContext::new(&question, &default_value, &question.help);
    let provider = get_prompt_provider();
    let prompt_handler = handler::PromptHandler::new(provider);
    let result = prompt_handler.create_prompt(&context)?;

    Ok(result.as_bool().unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPromptProvider;

    impl TextPrompter for TestPromptProvider {
        fn prompt_text(&self, _config: &TextPromptConfig) -> Result<String> {
            Ok("test".to_string())
        }
    }

    impl SingleChoicePrompter for TestPromptProvider {
        fn prompt_single_choice(&self, _config: &SingleChoiceConfig) -> Result<usize> {
            Ok(0)
        }
    }

    impl MultipleChoicePrompter for TestPromptProvider {
        fn prompt_multiple_choice(
            &self,
            _config: &MultipleChoiceConfig,
        ) -> Result<Vec<usize>> {
            Ok(vec![])
        }
    }

    impl ConfirmationPrompter for TestPromptProvider {
        fn prompt_confirmation(&self, _config: &ConfirmationConfig) -> Result<bool> {
            Ok(true)
        }
    }

    impl StructuredDataPrompter for TestPromptProvider {
        fn prompt_structured_data(
            &self,
            _config: &StructuredDataConfig,
        ) -> Result<serde_json::Value> {
            Ok(serde_json::Value::Null)
        }
    }

    impl<'a> Prompter<'a> for TestPromptProvider {
        fn prompt(&self, context: &PromptContext<'a>) -> Result<serde_json::Value> {
            match context.question.r#type {
                Type::Bool => {
                    let config = ConfirmationConfig {
                        prompt: context.help.to_string(),
                        default: context.default.as_bool().unwrap_or(false),
                    };
                    self.prompt_confirmation(&config).map(serde_json::Value::Bool)
                }
                _ => Ok(serde_json::Value::Null),
            }
        }
    }

    #[test]
    fn test_custom_prompt_provider() {
        let provider = TestPromptProvider;
        let question = Question {
            help: "Test?".to_string(),
            r#type: Type::Bool,
            default: serde_json::Value::Bool(false),
            choices: vec![],
            multiselect: false,
            secret: None,
            ask_if: String::new(),
            schema: None,
            validation: crate::config::types::get_default_validation(),
        };
        let context =
            PromptContext::new(&question, &serde_json::Value::Bool(false), "Help");
        let result = provider.prompt(&context);
        assert_eq!(result.unwrap(), serde_json::Value::Bool(true));
    }

    #[test]
    fn test_text_prompt_provider() {
        let provider = TestPromptProvider;
        let config = TextPromptConfig {
            prompt: "Enter text".to_string(),
            default: Some("default".to_string()),
            secret: None,
        };
        let result = provider.prompt_text(&config);
        assert_eq!(result.unwrap(), "test");
    }

    #[test]
    fn test_single_choice_prompt_provider() {
        let provider = TestPromptProvider;
        let config = SingleChoiceConfig {
            prompt: "Choose one".to_string(),
            choices: vec!["A".to_string(), "B".to_string()],
            default_index: Some(0),
        };
        let result = provider.prompt_single_choice(&config);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_multiple_choice_prompt_provider() {
        let provider = TestPromptProvider;
        let config = MultipleChoiceConfig {
            prompt: "Choose multiple".to_string(),
            choices: vec!["A".to_string(), "B".to_string()],
            defaults: vec![false, true],
        };
        let result = provider.prompt_multiple_choice(&config);
        assert_eq!(result.unwrap(), Vec::<usize>::new());
    }

    #[test]
    fn test_confirmation_prompt_provider() {
        let provider = TestPromptProvider;
        let config =
            ConfirmationConfig { prompt: "Are you sure?".to_string(), default: true };
        let result = provider.prompt_confirmation(&config);
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_structured_data_prompt_provider() {
        let provider = TestPromptProvider;
        let config = StructuredDataConfig {
            prompt: "Enter data".to_string(),
            default_value: serde_json::Value::Null,
            is_yaml: false,
            file_extension: "json".to_string(),
        };
        let result = provider.prompt_structured_data(&config);
        assert_eq!(result.unwrap(), serde_json::Value::Null);
    }
}
