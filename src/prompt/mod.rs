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

    let question = Question {
        help: prompt,
        r#type: Type::Bool,
        default: serde_json::Value::Bool(false),
        choices: Vec::new(),
        multiselect: false,
        secret: None,
        ask_if: String::new(),
        schema: None,
        validation: crate::config::Validation {
            condition: String::new(),
            error_message: "Invalid answer".to_string(),
        },
    };

    let default_value = serde_json::Value::Bool(false);
    let context = PromptContext::new(&question, &default_value, &question.help);
    let provider = get_prompt_provider();
    let prompt_handler = handler::PromptHandler::new(provider);
    let result = prompt_handler.create_prompt(&context)?;

    Ok(result.as_bool().unwrap_or(false))
}
