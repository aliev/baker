use crate::{
    config::{types::get_default_validation, Question, Type},
    error::Result,
};
use serde_json::Value;

use super::{
    context::PromptContext, dialoguer::DialoguerPrompter, handler::PromptHandler,
    interface::PromptProvider,
};

/// Trait implemented by prompt backends that can render a question via a [`PromptContext`].
pub trait Prompter<'a> {
    fn prompt(&self, prompt_context: &PromptContext<'a>) -> Result<Value>;
}

/// Convenience function to construct the default terminal prompt provider.
pub fn get_prompt_provider() -> impl PromptProvider {
    DialoguerPrompter::new()
}

/// High-level helper that collects an answer for a single configuration question.
///
/// # Examples
/// ```no_run
/// use baker::prompt::ask_question;
/// use serde_json::json;
///
/// # let question = baker::config::Question {
/// #     help: "Project name".into(),
/// #     r#type: baker::config::Type::Str,
/// #     default: json!("demo"),
/// #     choices: vec![],
/// #     multiselect: false,
/// #     secret: None,
/// #     ask_if: String::new(),
/// #     schema: None,
/// #     schema_file: None,
/// #     validation: baker::config::types::get_default_validation(),
/// # };
/// # let default = json!("demo");
/// let answer = ask_question(&question, &default, "Project name".to_string())?;
/// assert!(answer.is_string());
/// # Ok::<(), baker::error::Error>(())
/// ```
pub fn ask_question(question: &Question, default: &Value, help: String) -> Result<Value> {
    ask_question_with_provider(question, default, &help, get_prompt_provider())
}

/// Confirmation helper used for compatibility with legacy call sites.
pub fn confirm(skip: bool, prompt: String) -> Result<bool> {
    confirm_with_provider(skip, prompt, get_prompt_provider())
}

fn ask_question_with_provider<'a, P: PromptProvider>(
    question: &'a Question,
    default: &'a Value,
    help: &'a str,
    provider: P,
) -> Result<Value> {
    let context = PromptContext::new(question, default, help);
    let prompt_handler = PromptHandler::new(provider);
    prompt_handler.create_prompt(&context)
}

fn confirm_with_provider<P: PromptProvider>(
    skip: bool,
    prompt: String,
    provider: P,
) -> Result<bool> {
    if skip {
        return Ok(true);
    }

    let question = Question {
        help: prompt,
        r#type: Type::Bool,
        default: Value::Bool(false),
        choices: Vec::new(),
        multiselect: false,
        secret: None,
        ask_if: String::new(),
        schema: None,
        schema_file: None,
        validation: get_default_validation(),
    };

    let default_value = Value::Bool(false);
    let context = PromptContext::new(&question, &default_value, &question.help);
    let prompt_handler = PromptHandler::new(provider);
    let result = prompt_handler.create_prompt(&context)?;

    Ok(result.as_bool().unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::super::interface::{
        ConfirmationConfig, ConfirmationPrompter, MultipleChoiceConfig,
        MultipleChoicePrompter, SingleChoiceConfig, SingleChoicePrompter,
        StructuredDataConfig, StructuredDataPrompter, TextPromptConfig, TextPrompter,
    };
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
        ) -> Result<Value> {
            Ok(Value::Null)
        }
    }

    impl<'a> Prompter<'a> for TestPromptProvider {
        fn prompt(&self, context: &PromptContext<'a>) -> Result<Value> {
            match context.question.r#type {
                Type::Bool => {
                    let config = ConfirmationConfig {
                        prompt: context.help.to_string(),
                        default: context.default.as_bool().unwrap_or(false),
                    };
                    self.prompt_confirmation(&config).map(Value::Bool)
                }
                Type::Str => {
                    let config = TextPromptConfig {
                        prompt: context.help.to_string(),
                        default: Some(context.default.to_string()),
                        secret: None,
                    };
                    self.prompt_text(&config).map(Value::String)
                }
                _ => Ok(Value::Null),
            }
        }
    }

    #[test]
    fn ask_question_uses_injected_provider() {
        use crate::config::types::get_default_validation;
        let provider = TestPromptProvider;
        let question = Question {
            help: "Test?".to_string(),
            r#type: Type::Str,
            default: Value::String("ignored".into()),
            choices: vec![],
            multiselect: false,
            secret: None,
            ask_if: String::new(),
            schema: None,
            schema_file: None,
            validation: get_default_validation(),
        };
        let answer = super::ask_question_with_provider(
            &question,
            &Value::String("default".into()),
            "Help text",
            provider,
        )
        .unwrap();

        assert_eq!(answer, Value::String("test".into()));
    }

    #[test]
    fn confirm_returns_true_when_skipped() {
        let result =
            super::confirm_with_provider(true, "ignored".into(), TestPromptProvider)
                .unwrap();
        assert!(result);
    }

    #[test]
    fn confirm_uses_injected_provider() {
        let result =
            super::confirm_with_provider(false, "Proceed?".into(), TestPromptProvider)
                .unwrap();
        assert!(result);
    }

    #[test]
    fn test_custom_prompt_provider() {
        use crate::config::types::get_default_validation;
        let provider = TestPromptProvider;
        let question = Question {
            help: "Test?".to_string(),
            r#type: Type::Bool,
            default: Value::Bool(false),
            choices: vec![],
            multiselect: false,
            secret: None,
            ask_if: String::new(),
            schema: None,
            schema_file: None,
            validation: get_default_validation(),
        };
        let context = PromptContext::new(&question, &Value::Bool(false), "Help");
        let result = provider.prompt(&context);
        assert_eq!(result.unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_text_prompt_provider() {
        let provider = TestPromptProvider;
        let config = TextPromptConfig {
            prompt: "Enter text".to_string(),
            default: Some("default".to_string()),
            secret: None,
        };
        let result = TextPrompter::prompt_text(&provider, &config);
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
        let result = SingleChoicePrompter::prompt_single_choice(&provider, &config);
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
        let result = MultipleChoicePrompter::prompt_multiple_choice(&provider, &config);
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_structured_data_prompt_provider() {
        let provider = TestPromptProvider;
        let config = StructuredDataConfig {
            prompt: "Provide JSON".to_string(),
            default_value: Value::Null,
            is_yaml: false,
            file_extension: "json".to_string(),
        };
        let result = StructuredDataPrompter::prompt_structured_data(&provider, &config);
        assert_eq!(result.unwrap(), Value::Null);
    }
}
