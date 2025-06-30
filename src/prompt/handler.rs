//! Factory for creating and executing different types of prompts
//!
//! This module provides a unified interface for prompt creation and execution,
//! automatically selecting the appropriate prompt type based on input configuration.

use super::interface::{
    ConfirmationConfig, MultipleChoiceConfig, PromptProvider, SecretConfig,
    SingleChoiceConfig, StructuredDataConfig, TextPromptConfig,
};
use crate::{
    config::{IntoQuestionType, QuestionType},
    error::Result,
    prompt::PromptContext,
};
use serde_json::Value;

/// Creates and executes prompts based on context configuration
pub struct PromptHandler<P: PromptProvider> {
    provider: P,
}

impl<P: PromptProvider> PromptHandler<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Creates and executes a prompt based on the provided context
    pub fn create_prompt(&self, prompt_context: &PromptContext) -> Result<Value> {
        match prompt_context.question.into_question_type() {
            QuestionType::Text => self.prompt_text(prompt_context),
            QuestionType::SingleChoice => self.prompt_single_choice(prompt_context),
            QuestionType::MultipleChoice => self.prompt_multiple_choice(prompt_context),
            QuestionType::Boolean => self.prompt_confirmation(prompt_context),
            QuestionType::Json => self.prompt_structured_data(prompt_context, false),
            QuestionType::Yaml => self.prompt_structured_data(prompt_context, true),
        }
    }

    fn prompt_text(&self, prompt_context: &PromptContext) -> Result<Value> {
        let config = self.create_text_config(prompt_context);
        let result = self.provider.prompt_text(&config)?;
        Ok(Value::String(result))
    }

    fn prompt_single_choice(&self, prompt_context: &PromptContext) -> Result<Value> {
        let config = self.create_single_choice_config(prompt_context);
        let selection_index = self.provider.prompt_single_choice(&config)?;
        let selected_choice = &prompt_context.question.choices[selection_index];
        Ok(Value::String(selected_choice.clone()))
    }

    fn prompt_multiple_choice(&self, prompt_context: &PromptContext) -> Result<Value> {
        let config = self.create_multiple_choice_config(prompt_context);
        let indices = self.provider.prompt_multiple_choice(&config)?;

        let selected: Vec<Value> = indices
            .iter()
            .map(|&i| Value::String(prompt_context.question.choices[i].clone()))
            .collect();

        Ok(Value::Array(selected))
    }

    fn prompt_confirmation(&self, prompt_context: &PromptContext) -> Result<Value> {
        let config = self.create_confirmation_config(prompt_context);
        let result = self.provider.prompt_confirmation(&config)?;
        Ok(Value::Bool(result))
    }

    fn prompt_structured_data(
        &self,
        prompt_context: &PromptContext,
        is_yaml: bool,
    ) -> Result<Value> {
        let config = self.create_structured_data_config(prompt_context, is_yaml);
        self.provider.prompt_structured_data(&config)
    }

    // Configuration creation methods

    fn create_text_config(&self, prompt_context: &PromptContext) -> TextPromptConfig {
        let default = self.value_to_default_string(prompt_context.default);
        let secret = prompt_context.question.secret.as_ref().map(|s| SecretConfig {
            confirm: s.confirm,
            mismatch_error: if s.mistmatch_err.is_empty() {
                "Mismatch".to_string()
            } else {
                s.mistmatch_err.clone()
            },
        });

        TextPromptConfig {
            prompt: prompt_context.help.to_string(),
            default: if default.is_empty() { None } else { Some(default) },
            secret,
        }
    }

    fn create_single_choice_config(
        &self,
        prompt_context: &PromptContext,
    ) -> SingleChoiceConfig {
        let default_index = self.find_default_choice_index(
            &prompt_context.question.choices,
            prompt_context.default,
        );

        SingleChoiceConfig {
            prompt: prompt_context.help.to_string(),
            choices: prompt_context.question.choices.clone(),
            default_index: if default_index > 0 { Some(default_index) } else { None },
        }
    }

    fn create_multiple_choice_config(
        &self,
        prompt_context: &PromptContext,
    ) -> MultipleChoiceConfig {
        let default_strings = self.extract_string_array(prompt_context.default);
        let defaults = self
            .create_choice_defaults(&prompt_context.question.choices, &default_strings);

        MultipleChoiceConfig {
            prompt: prompt_context.help.to_string(),
            choices: prompt_context.question.choices.clone(),
            defaults,
        }
    }

    fn create_confirmation_config(
        &self,
        prompt_context: &PromptContext,
    ) -> ConfirmationConfig {
        let default = prompt_context.default.as_bool().unwrap_or(false);

        ConfirmationConfig { prompt: prompt_context.help.to_string(), default }
    }

    fn create_structured_data_config(
        &self,
        prompt_context: &PromptContext,
        is_yaml: bool,
    ) -> StructuredDataConfig {
        let file_extension = if is_yaml { ".yaml" } else { ".json" };

        StructuredDataConfig {
            prompt: prompt_context.help.to_string(),
            default_value: prompt_context.default.clone(),
            is_yaml,
            file_extension: file_extension.to_string(),
        }
    }

    // Helper methods

    fn value_to_default_string(&self, value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            _ => value.to_string(),
        }
    }

    fn find_default_choice_index(
        &self,
        choices: &[String],
        default_value: &Value,
    ) -> usize {
        match default_value {
            Value::String(default_str) => {
                choices.iter().position(|choice| choice == default_str).unwrap_or(0)
            }
            _ => 0,
        }
    }

    fn extract_string_array(&self, value: &Value) -> Vec<String> {
        match value {
            Value::Array(arr) => arr
                .iter()
                .filter_map(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    fn create_choice_defaults(
        &self,
        choices: &[String],
        default_strings: &[String],
    ) -> Vec<bool> {
        choices.iter().map(|choice| default_strings.contains(choice)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Question, Secret, Type, Validation};
    use crate::prompt::interface::{
        ConfirmationPrompter, MultipleChoicePrompter, SingleChoicePrompter,
        StructuredDataPrompter, TextPrompter,
    };
    use serde_json::json;
    use std::cell::RefCell;

    /// Mock provider for testing
    #[derive(Debug, Default)]
    struct MockProvider {
        text_responses: RefCell<Vec<String>>,
        single_choice_responses: RefCell<Vec<usize>>,
        multiple_choice_responses: RefCell<Vec<Vec<usize>>>,
        confirmation_responses: RefCell<Vec<bool>>,
        structured_data_responses: RefCell<Vec<Value>>,

        // Track calls for verification
        text_calls: RefCell<Vec<TextPromptConfig>>,
        single_choice_calls: RefCell<Vec<SingleChoiceConfig>>,
        multiple_choice_calls: RefCell<Vec<MultipleChoiceConfig>>,
        confirmation_calls: RefCell<Vec<ConfirmationConfig>>,
        structured_data_calls: RefCell<Vec<StructuredDataConfig>>,
    }

    impl MockProvider {
        fn new() -> Self {
            Default::default()
        }

        fn with_text_response(self, response: String) -> Self {
            self.text_responses.borrow_mut().push(response);
            self
        }

        fn with_single_choice_response(self, response: usize) -> Self {
            self.single_choice_responses.borrow_mut().push(response);
            self
        }

        fn with_multiple_choice_response(self, response: Vec<usize>) -> Self {
            self.multiple_choice_responses.borrow_mut().push(response);
            self
        }

        fn with_confirmation_response(self, response: bool) -> Self {
            self.confirmation_responses.borrow_mut().push(response);
            self
        }

        fn with_structured_data_response(self, response: Value) -> Self {
            self.structured_data_responses.borrow_mut().push(response);
            self
        }

        fn get_text_calls(&self) -> Vec<TextPromptConfig> {
            self.text_calls.borrow().clone()
        }

        fn get_single_choice_calls(&self) -> Vec<SingleChoiceConfig> {
            self.single_choice_calls.borrow().clone()
        }

        fn get_multiple_choice_calls(&self) -> Vec<MultipleChoiceConfig> {
            self.multiple_choice_calls.borrow().clone()
        }

        fn get_confirmation_calls(&self) -> Vec<ConfirmationConfig> {
            self.confirmation_calls.borrow().clone()
        }

        fn get_structured_data_calls(&self) -> Vec<StructuredDataConfig> {
            self.structured_data_calls.borrow().clone()
        }
    }

    impl TextPrompter for MockProvider {
        fn prompt_text(&self, config: &TextPromptConfig) -> Result<String> {
            self.text_calls.borrow_mut().push(config.clone());
            Ok(self.text_responses.borrow_mut().remove(0))
        }
    }

    impl SingleChoicePrompter for MockProvider {
        fn prompt_single_choice(&self, config: &SingleChoiceConfig) -> Result<usize> {
            self.single_choice_calls.borrow_mut().push(config.clone());
            Ok(self.single_choice_responses.borrow_mut().remove(0))
        }
    }

    impl MultipleChoicePrompter for MockProvider {
        fn prompt_multiple_choice(
            &self,
            config: &MultipleChoiceConfig,
        ) -> Result<Vec<usize>> {
            self.multiple_choice_calls.borrow_mut().push(config.clone());
            Ok(self.multiple_choice_responses.borrow_mut().remove(0))
        }
    }

    impl ConfirmationPrompter for MockProvider {
        fn prompt_confirmation(&self, config: &ConfirmationConfig) -> Result<bool> {
            self.confirmation_calls.borrow_mut().push(config.clone());
            Ok(self.confirmation_responses.borrow_mut().remove(0))
        }
    }

    impl StructuredDataPrompter for MockProvider {
        fn prompt_structured_data(&self, config: &StructuredDataConfig) -> Result<Value> {
            self.structured_data_calls.borrow_mut().push(config.clone());
            Ok(self.structured_data_responses.borrow_mut().remove(0))
        }
    }

    fn create_test_validation() -> Validation {
        Validation {
            condition: "true".to_string(),
            error_message: "Invalid answer".to_string(),
        }
    }

    fn create_text_question() -> Question {
        Question {
            help: "Enter your name".to_string(),
            r#type: Type::Str,
            default: json!("John"),
            choices: vec![],
            multiselect: false,
            secret: None,
            ask_if: String::new(),
            schema: None,
            validation: create_test_validation(),
        }
    }

    fn create_secret_question() -> Question {
        Question {
            help: "Enter password".to_string(),
            r#type: Type::Str,
            default: Value::Null,
            choices: vec![],
            multiselect: false,
            secret: Some(Secret {
                confirm: true,
                mistmatch_err: "Passwords don't match".to_string(),
            }),
            ask_if: String::new(),
            schema: None,
            validation: create_test_validation(),
        }
    }

    fn create_single_choice_question() -> Question {
        Question {
            help: "Choose your favorite color".to_string(),
            r#type: Type::Str,
            default: json!("blue"),
            choices: vec!["red".to_string(), "blue".to_string(), "green".to_string()],
            multiselect: false,
            secret: None,
            ask_if: String::new(),
            schema: None,
            validation: create_test_validation(),
        }
    }

    fn create_multiple_choice_question() -> Question {
        Question {
            help: "Select languages you know".to_string(),
            r#type: Type::Str,
            default: json!(["rust", "python"]),
            choices: vec![
                "rust".to_string(),
                "python".to_string(),
                "go".to_string(),
                "java".to_string(),
            ],
            multiselect: true,
            secret: None,
            ask_if: String::new(),
            schema: None,
            validation: create_test_validation(),
        }
    }

    fn create_boolean_question() -> Question {
        Question {
            help: "Do you want to continue?".to_string(),
            r#type: Type::Bool,
            default: json!(true),
            choices: vec![],
            multiselect: false,
            secret: None,
            ask_if: String::new(),
            schema: None,
            validation: create_test_validation(),
        }
    }

    fn create_json_question() -> Question {
        Question {
            help: "Enter JSON data".to_string(),
            r#type: Type::Json,
            default: json!({"key": "value"}),
            choices: vec![],
            multiselect: false,
            secret: None,
            ask_if: String::new(),
            schema: None,
            validation: create_test_validation(),
        }
    }

    fn create_yaml_question() -> Question {
        Question {
            help: "Enter YAML data".to_string(),
            r#type: Type::Yaml,
            default: json!({"key": "value"}),
            choices: vec![],
            multiselect: false,
            secret: None,
            ask_if: String::new(),
            schema: None,
            validation: create_test_validation(),
        }
    }

    #[test]
    fn test_prompt_text_basic() {
        let mock = MockProvider::new().with_text_response("Alice".to_string());
        let prompt_handler = PromptHandler::new(mock);

        let question = create_text_question();
        let default_value = json!("John");
        let context = PromptContext::new(&question, &default_value, "Enter your name");

        let result = prompt_handler.create_prompt(&context).unwrap();
        assert_eq!(result, Value::String("Alice".to_string()));

        let calls = prompt_handler.provider.get_text_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].prompt, "Enter your name");
        assert_eq!(calls[0].default, Some("John".to_string()));
        assert!(calls[0].secret.is_none());
    }

    #[test]
    fn test_prompt_text_with_secret() {
        let mock = MockProvider::new().with_text_response("secret123".to_string());
        let prompt_handler = PromptHandler::new(mock);

        let question = create_secret_question();
        let context = PromptContext::new(&question, &Value::Null, "Enter password");

        let result = prompt_handler.create_prompt(&context).unwrap();
        assert_eq!(result, Value::String("secret123".to_string()));

        let calls = prompt_handler.provider.get_text_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].prompt, "Enter password");
        assert_eq!(calls[0].default, None);
        assert!(calls[0].secret.is_some());

        let secret = calls[0].secret.as_ref().unwrap();
        assert!(secret.confirm);
        assert_eq!(secret.mismatch_error, "Passwords don't match");
    }

    #[test]
    fn test_prompt_text_empty_default() {
        let mock = MockProvider::new().with_text_response("response".to_string());
        let prompt_handler = PromptHandler::new(mock);

        let question = create_text_question();
        let context = PromptContext::new(&question, &Value::Null, "Enter text");

        let result = prompt_handler.create_prompt(&context).unwrap();
        assert_eq!(result, Value::String("response".to_string()));

        let calls = prompt_handler.provider.get_text_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].default, None);
    }

    #[test]
    fn test_prompt_single_choice() {
        let mock = MockProvider::new().with_single_choice_response(1);
        let prompt_handler = PromptHandler::new(mock);

        let question = create_single_choice_question();
        let default_value = json!("blue");
        let context =
            PromptContext::new(&question, &default_value, "Choose your favorite color");

        let result = prompt_handler.create_prompt(&context).unwrap();
        assert_eq!(result, Value::String("blue".to_string()));

        let calls = prompt_handler.provider.get_single_choice_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].prompt, "Choose your favorite color");
        assert_eq!(calls[0].choices, vec!["red", "blue", "green"]);
        assert_eq!(calls[0].default_index, Some(1));
    }

    #[test]
    fn test_prompt_single_choice_no_default() {
        let mock = MockProvider::new().with_single_choice_response(0);
        let prompt_handler = PromptHandler::new(mock);

        let question = create_single_choice_question();
        let context =
            PromptContext::new(&question, &Value::Null, "Choose your favorite color");

        let result = prompt_handler.create_prompt(&context).unwrap();
        assert_eq!(result, Value::String("red".to_string()));

        let calls = prompt_handler.provider.get_single_choice_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].default_index, None);
    }

    #[test]
    fn test_prompt_multiple_choice() {
        let mock = MockProvider::new().with_multiple_choice_response(vec![0, 1]);
        let prompt_handler = PromptHandler::new(mock);

        let question = create_multiple_choice_question();
        let default_value = json!(["rust", "python"]);
        let context =
            PromptContext::new(&question, &default_value, "Select languages you know");

        let result = prompt_handler.create_prompt(&context).unwrap();
        assert_eq!(
            result,
            Value::Array(vec![
                Value::String("rust".to_string()),
                Value::String("python".to_string())
            ])
        );

        let calls = prompt_handler.provider.get_multiple_choice_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].prompt, "Select languages you know");
        assert_eq!(calls[0].choices, vec!["rust", "python", "go", "java"]);
        assert_eq!(calls[0].defaults, vec![true, true, false, false]);
    }

    #[test]
    fn test_prompt_multiple_choice_empty_defaults() {
        let mock = MockProvider::new().with_multiple_choice_response(vec![2]);
        let prompt_handler = PromptHandler::new(mock);

        let question = create_multiple_choice_question();
        let context =
            PromptContext::new(&question, &Value::Null, "Select languages you know");

        let result = prompt_handler.create_prompt(&context).unwrap();
        assert_eq!(result, Value::Array(vec![Value::String("go".to_string())]));

        let calls = prompt_handler.provider.get_multiple_choice_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].defaults, vec![false, false, false, false]);
    }

    #[test]
    fn test_prompt_confirmation_true() {
        let mock = MockProvider::new().with_confirmation_response(true);
        let prompt_handler = PromptHandler::new(mock);

        let question = create_boolean_question();
        let context =
            PromptContext::new(&question, &json!(true), "Do you want to continue?");

        let result = prompt_handler.create_prompt(&context).unwrap();
        assert_eq!(result, Value::Bool(true));

        let calls = prompt_handler.provider.get_confirmation_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].prompt, "Do you want to continue?");
        assert_eq!(calls[0].default, true);
    }

    #[test]
    fn test_prompt_confirmation_false() {
        let mock = MockProvider::new().with_confirmation_response(false);
        let prompt_handler = PromptHandler::new(mock);

        let question = create_boolean_question();
        let context =
            PromptContext::new(&question, &json!(false), "Do you want to continue?");

        let result = prompt_handler.create_prompt(&context).unwrap();
        assert_eq!(result, Value::Bool(false));

        let calls = prompt_handler.provider.get_confirmation_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].default, false);
    }

    #[test]
    fn test_prompt_confirmation_null_default() {
        let mock = MockProvider::new().with_confirmation_response(true);
        let prompt_handler = PromptHandler::new(mock);

        let question = create_boolean_question();
        let context =
            PromptContext::new(&question, &Value::Null, "Do you want to continue?");

        let result = prompt_handler.create_prompt(&context).unwrap();
        assert_eq!(result, Value::Bool(true));

        let calls = prompt_handler.provider.get_confirmation_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].default, false); // should default to false for null
    }

    #[test]
    fn test_prompt_structured_data_json() {
        let response_data = json!({"name": "test", "value": 42});
        let mock =
            MockProvider::new().with_structured_data_response(response_data.clone());
        let prompt_handler = PromptHandler::new(mock);

        let question = create_json_question();
        let default_value = json!({"key": "value"});
        let context = PromptContext::new(&question, &default_value, "Enter JSON data");

        let result = prompt_handler.create_prompt(&context).unwrap();
        assert_eq!(result, response_data);

        let calls = prompt_handler.provider.get_structured_data_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].prompt, "Enter JSON data");
        assert_eq!(calls[0].default_value, json!({"key": "value"}));
        assert!(!calls[0].is_yaml);
        assert_eq!(calls[0].file_extension, ".json");
    }

    #[test]
    fn test_prompt_structured_data_yaml() {
        let response_data = json!({"name": "test", "value": 42});
        let mock =
            MockProvider::new().with_structured_data_response(response_data.clone());
        let prompt_handler = PromptHandler::new(mock);

        let question = create_yaml_question();
        let default_value = json!({"key": "value"});
        let context = PromptContext::new(&question, &default_value, "Enter YAML data");

        let result = prompt_handler.create_prompt(&context).unwrap();
        assert_eq!(result, response_data);

        let calls = prompt_handler.provider.get_structured_data_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].prompt, "Enter YAML data");
        assert_eq!(calls[0].default_value, json!({"key": "value"}));
        assert!(calls[0].is_yaml);
        assert_eq!(calls[0].file_extension, ".yaml");
    }

    #[test]
    fn test_value_to_default_string() {
        let mock = MockProvider::new();
        let prompt_handler = PromptHandler::new(mock);

        assert_eq!(prompt_handler.value_to_default_string(&json!("test")), "test");
        assert_eq!(prompt_handler.value_to_default_string(&Value::Null), "");
        assert_eq!(prompt_handler.value_to_default_string(&json!(42)), "42");
        assert_eq!(prompt_handler.value_to_default_string(&json!(true)), "true");
        assert_eq!(
            prompt_handler.value_to_default_string(&json!({"key": "value"})),
            r#"{"key":"value"}"#
        );
    }

    #[test]
    fn test_find_default_choice_index() {
        let mock = MockProvider::new();
        let prompt_handler = PromptHandler::new(mock);

        let choices = vec!["red".to_string(), "blue".to_string(), "green".to_string()];

        assert_eq!(prompt_handler.find_default_choice_index(&choices, &json!("blue")), 1);
        assert_eq!(prompt_handler.find_default_choice_index(&choices, &json!("red")), 0);
        assert_eq!(
            prompt_handler.find_default_choice_index(&choices, &json!("yellow")),
            0
        );
        assert_eq!(prompt_handler.find_default_choice_index(&choices, &Value::Null), 0);
        assert_eq!(prompt_handler.find_default_choice_index(&choices, &json!(42)), 0);
    }

    #[test]
    fn test_extract_string_array() {
        let mock = MockProvider::new();
        let prompt_handler = PromptHandler::new(mock);

        assert_eq!(
            prompt_handler.extract_string_array(&json!(["a", "b", "c"])),
            vec!["a", "b", "c"]
        );
        assert_eq!(
            prompt_handler.extract_string_array(&json!(["a", 42, "c"])),
            vec!["a", "c"] // Should skip non-string values
        );
        assert_eq!(
            prompt_handler.extract_string_array(&Value::Null),
            Vec::<String>::new()
        );
        assert_eq!(
            prompt_handler.extract_string_array(&json!("not an array")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_create_choice_defaults() {
        let mock = MockProvider::new();
        let prompt_handler = PromptHandler::new(mock);

        let choices = vec!["rust".to_string(), "python".to_string(), "go".to_string()];
        let defaults = vec!["rust".to_string(), "go".to_string()];

        let result = prompt_handler.create_choice_defaults(&choices, &defaults);
        assert_eq!(result, vec![true, false, true]);
    }

    #[test]
    fn test_secret_config_with_empty_mismatch_error() {
        let mock = MockProvider::new().with_text_response("password".to_string());
        let prompt_handler = PromptHandler::new(mock);

        let mut question = create_secret_question();
        question.secret = Some(Secret {
            confirm: true,
            mistmatch_err: String::new(), // Empty error message
        });

        let context = PromptContext::new(&question, &Value::Null, "Enter password");

        let _result = prompt_handler.create_prompt(&context).unwrap();

        let calls = prompt_handler.provider.get_text_calls();
        assert_eq!(calls.len(), 1);

        let secret = calls[0].secret.as_ref().unwrap();
        assert_eq!(secret.mismatch_error, "Mismatch"); // Should use default
    }

    #[test]
    fn test_prompt_handler_new() {
        let mock = MockProvider::new();
        let prompt_handler = PromptHandler::new(mock);

        // Just verify we can create the prompt_handler
        // The actual functionality is tested in other tests
        assert_eq!(
            std::mem::size_of_val(&prompt_handler),
            std::mem::size_of::<MockProvider>()
        );
    }
}
