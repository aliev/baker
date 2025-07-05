use crate::{
    config::{ConfigV1, IntoQuestionType, Question, QuestionRendered, QuestionType},
    error::{Error, Result},
    prompt::ask_question,
    renderer::TemplateRenderer,
};
use serde_json::{json, Map, Value};

/// Collects answers from various sources: pre-hook output, command line arguments, and user prompts
pub struct AnswerCollector<'a> {
    engine: &'a dyn TemplateRenderer,
    non_interactive: bool,
}

#[derive(Debug)]
pub enum ValidationError {
    JsonSchema(String),
    FieldValidation(String),
}

impl<'a> AnswerCollector<'a> {
    pub fn new(engine: &'a dyn TemplateRenderer, non_interactive: bool) -> Self {
        Self { engine, non_interactive }
    }

    /// Read content from a reader into a string.
    fn read_from(&self, mut reader: impl std::io::Read) -> Result<String> {
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        Ok(buf)
    }

    /// Collects answers from all available sources
    pub fn collect_answers(
        &self,
        config: &ConfigV1,
        pre_hook_output: Option<String>,
        cli_answers: Option<String>,
    ) -> Result<Value> {
        let mut answers = Map::new();

        // Add answers from pre-hook output
        if let Some(result) = pre_hook_output {
            log::debug!(
                "Pre-hook stdout content (attempting to parse as JSON answers): {result}"
            );

            let pre_answers = serde_json::from_str::<Value>(&result).map_or_else(
                |e| {
                    log::warn!("Failed to parse hook output as JSON: {e}");
                    Map::new()
                },
                |value| match value {
                    Value::Object(map) => map,
                    _ => Map::new(),
                },
            );
            answers.extend(pre_answers);
        }

        // Add answers from command line arguments
        if let Some(answers_arg) = cli_answers {
            let answers_str = if answers_arg == "-" {
                self.read_from(std::io::stdin())?
            } else {
                answers_arg
            };
            let cli_answers = self.parse_string_to_json(answers_str)?;
            answers.extend(cli_answers);
        }

        // Collect answers for each question through interactive prompts
        for (key, question) in &config.questions {
            self.collect_question_answer(&mut answers, key, question)?;
        }

        Ok(Value::Object(answers))
    }

    /// Collects answer for a single question
    fn collect_question_answer(
        &self,
        answers: &mut Map<String, Value>,
        key: &str,
        question: &crate::config::Question,
    ) -> Result<()> {
        loop {
            let QuestionRendered { help, default, ask_if, .. } =
                question.render(key, &json!(answers), self.engine);

            // Determine if we should skip interactive prompting based on:
            // 1. User explicitly requested non-interactive mode with --non-interactive flag, OR
            // 2. The template's ask_if condition evaluated to false for this question
            let skip_user_prompt = self.non_interactive || !ask_if;

            if skip_user_prompt {
                // Skip to the next question if an answer for this key is already provided
                if answers.contains_key(key) {
                    break;
                }

                // Use the template's default value if one was specified
                if !question.default.is_null() {
                    answers.insert(key.to_string(), default.clone());
                    break;
                }
            }

            let answer = match ask_question(question, &default, help) {
                Ok(answer) => answer,
                Err(err) => match err {
                    Error::JSONParseError(_) | Error::YAMLParseError(_) => {
                        println!("{err}");
                        continue;
                    }
                    _ => return Err(err),
                },
            };

            answers.insert(key.to_string(), answer.clone());
            let _answers = Value::Object(answers.clone());

            match self.validate_answer(question, &answer, self.engine, &_answers) {
                Ok(_) => break,
                Err(err) => match err {
                    ValidationError::JsonSchema(msg) => println!("{msg}"),
                    ValidationError::FieldValidation(msg) => println!("{msg}"),
                },
            }
        }

        Ok(())
    }

    /// Parse a string into a JSON object.
    fn parse_string_to_json(
        &self,
        buf: String,
    ) -> Result<serde_json::Map<String, serde_json::Value>> {
        let value = serde_json::from_str(&buf)?;

        match value {
            serde_json::Value::Object(map) => Ok(map),
            _ => Ok(serde_json::Map::new()),
        }
    }

    fn validate_answer(
        &self,
        question: &Question,
        answer: &serde_json::Value,
        engine: &dyn TemplateRenderer,
        answers: &serde_json::Value,
    ) -> Result<(), ValidationError> {
        match question.into_question_type() {
            QuestionType::Json | QuestionType::Yaml => {
                if let Some(schema) = &question.schema {
                    self.validate_with_schema(answer, schema).map_err(|e| {
                        ValidationError::JsonSchema(format!(
                            "JSON Schema validation error: {e}"
                        ))
                    })?;
                }
            }
            _ => {
                let is_valid = engine
                    .execute_expression(&question.validation.condition, answers)
                    .unwrap_or(true);

                if !is_valid {
                    let error_message = engine
                        .render(
                            &question.validation.error_message,
                            answers,
                            Some("validation_error"),
                        )
                        .unwrap_or_else(|_| "Validation failed".to_string());
                    return Err(ValidationError::FieldValidation(error_message));
                }
            }
        }
        Ok(())
    }

    /// Validate a value against a JSON schema.
    fn validate_with_schema(
        &self,
        value: &serde_json::Value,
        schema: &str,
    ) -> Result<()> {
        let schema_value: serde_json::Value = serde_json::from_str(schema)?;

        let validator = jsonschema::validator_for(&schema_value).map_err(|e| {
            crate::error::Error::Other(anyhow::anyhow!("Invalid JSON schema: {}", e))
        })?;

        let errors: Vec<String> = validator
            .iter_errors(value)
            .map(|error| format!("Error: {} (at {})", error, error.instance_path))
            .collect();

        if !errors.is_empty() {
            return Err(crate::error::Error::Other(anyhow::anyhow!(
                "Validation failed: {}",
                errors.join("\n")
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
impl<'a> AnswerCollector<'a> {
    /// Test helper method to access validate_with_schema
    pub fn test_validate_with_schema(
        &self,
        value: &serde_json::Value,
        schema: &str,
    ) -> Result<()> {
        self.validate_with_schema(value, schema)
    }

    /// Test helper method to access validate_answer
    pub fn test_validate_answer(
        &self,
        question: &Question,
        answer: &serde_json::Value,
        engine: &dyn TemplateRenderer,
        answers: &serde_json::Value,
    ) -> Result<(), ValidationError> {
        self.validate_answer(question, answer, engine, answers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Type, Validation};
    use crate::template::get_template_engine;
    use serde_json::json;

    #[test]
    fn test_validate_with_schema_invalid_schema() {
        let engine = get_template_engine();
        let collector = AnswerCollector::new(&engine, false);
        let value = json!({"name": "test"});
        let invalid_schema =
            r#"{"type": "object", "properties": {"name": {"type": "string"}}"#;
        assert!(collector.test_validate_with_schema(&value, invalid_schema).is_err());
    }
    #[test]
    fn test_validate_with_schema_valid_value() {
        let engine = get_template_engine();
        let collector = AnswerCollector::new(&engine, false);
        let value = json!({"name": "test"});
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}"#;
        assert!(collector.test_validate_with_schema(&value, schema).is_ok());
    }

    #[test]
    fn test_validate_with_schema_invalid_value() {
        let engine = get_template_engine();
        let collector = AnswerCollector::new(&engine, false);
        let value = json!({"name": 123});
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}"#;
        assert!(collector.test_validate_with_schema(&value, schema).is_err());
    }

    fn make_question_json(
        schema: Option<String>,
        condition: &str,
        error_message: &str,
    ) -> Question {
        Question {
            help: String::new(),
            r#type: Type::Json,
            default: serde_json::Value::Null,
            choices: vec![],
            multiselect: false,
            secret: None,
            ask_if: String::new(),
            schema,
            validation: Validation {
                condition: condition.to_string(),
                error_message: error_message.to_string(),
            },
        }
    }

    #[test]
    fn test_validate_answer_json_schema_valid() {
        let question = make_question_json(
                Some(r#"{"type": "object", "properties": {"foo": {"type": "string"}}, "required": ["foo"]}"#.to_string()),
                "true",
                "error"
            );
        let answer = json!({"foo": "bar"});
        let engine = get_template_engine();
        let collector = AnswerCollector::new(&engine, false);
        let answers = json!({});
        assert!(collector
            .test_validate_answer(&question, &answer, &engine, &answers)
            .is_ok());
    }

    #[test]
    fn test_validate_answer_json_schema_invalid() {
        let question = make_question_json(
                Some(r#"{"type": "object", "properties": {"foo": {"type": "string"}}, "required": ["foo"]}"#.to_string()),
                "true",
                "error"
            );
        let answer = json!({"foo": 123});
        let engine = get_template_engine();
        let collector = AnswerCollector::new(&engine, false);
        let answers = json!({});
        assert!(matches!(
            collector.test_validate_answer(&question, &answer, &engine, &answers),
            Err(ValidationError::JsonSchema(_))
        ));
    }

    #[test]
    fn test_validate_answer_field_validation_valid() {
        let question = make_question_json(None, "true", "error");
        let answer = json!("anything");
        let engine = get_template_engine();
        let collector = AnswerCollector::new(&engine, false);
        let answers = json!({});
        assert!(collector
            .test_validate_answer(&question, &answer, &engine, &answers)
            .is_ok());
    }

    #[test]
    fn test_validate_answer_field_validation_invalid() {
        let question = Question {
            help: String::new(),
            r#type: Type::Str,
            default: serde_json::Value::Null,
            choices: vec![],
            multiselect: false,
            secret: None,
            ask_if: String::new(),
            schema: None,
            validation: Validation {
                condition: "false".to_string(),
                error_message: "custom error".to_string(),
            },
        };

        let answer = serde_json::json!("anything");
        let engine = get_template_engine();
        let collector = AnswerCollector::new(&engine, false);
        let answers = serde_json::json!({});
        let err = collector
            .test_validate_answer(&question, &answer, &engine, &answers)
            .unwrap_err();
        match err {
            ValidationError::FieldValidation(msg) => assert_eq!(msg, "custom error"),
            _ => panic!("Expected FieldValidation error"),
        }
    }
}
