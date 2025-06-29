use crate::{
    config::{IntoQuestionType, Question, QuestionType},
    error::Result,
    renderer::TemplateRenderer,
};

#[derive(Debug)]
pub enum ValidationError {
    JsonSchema(String),
    FieldValidation(String),
}

/// Validate a value against a JSON schema.
pub fn validate_with_schema(value: &serde_json::Value, schema: &str) -> Result<()> {
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

pub fn validate_answer(
    question: &Question,
    answer: &serde_json::Value,
    engine: &dyn TemplateRenderer,
    answers: &serde_json::Value,
) -> Result<(), ValidationError> {
    match question.into_question_type() {
        QuestionType::Json | QuestionType::Yaml => {
            if let Some(schema) = &question.schema {
                validate_with_schema(answer, schema).map_err(|e| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Type, Validation};
    use crate::template::get_template_engine;
    use serde_json::json;

    #[test]
    fn test_validate_with_schema_invalid_schema() {
        let value = json!({"name": "test"});
        let invalid_schema =
            r#"{"type": "object", "properties": {"name": {"type": "string"}}"#;
        assert!(validate_with_schema(&value, invalid_schema).is_err());
    }
    #[test]
    fn test_validate_with_schema_valid_value() {
        let value = json!({"name": "test"});
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}"#;
        assert!(validate_with_schema(&value, schema).is_ok());
    }

    #[test]
    fn test_validate_with_schema_invalid_value() {
        let value = json!({"name": 123});
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}"#;
        assert!(validate_with_schema(&value, schema).is_err());
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
        let answers = json!({});
        assert!(validate_answer(&question, &answer, &engine, &answers).is_ok());
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
        let answers = json!({});
        assert!(matches!(
            validate_answer(&question, &answer, &engine, &answers),
            Err(ValidationError::JsonSchema(_))
        ));
    }

    #[test]
    fn test_validate_answer_field_validation_valid() {
        let question = make_question_json(None, "true", "error");
        let answer = json!("anything");
        let engine = get_template_engine();
        let answers = json!({});
        assert!(validate_answer(&question, &answer, &engine, &answers).is_ok());
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
        let answers = serde_json::json!({});
        let err = validate_answer(&question, &answer, &engine, &answers).unwrap_err();
        match err {
            ValidationError::FieldValidation(msg) => assert_eq!(msg, "custom error"),
            _ => panic!("Expected FieldValidation error"),
        }
    }
}
