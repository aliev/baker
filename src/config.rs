use crate::error::{Error, Result};
use crate::ioutils::path_to_str;
use crate::renderer::TemplateRenderer;
use indexmap::IndexMap;
use serde::Deserialize;
use std::path::Path;

pub const CONFIG_LIST: &[&str] = &["baker.json", "baker.yaml", "baker.yml"];

/// Type of question to be presented to the user
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    /// String input question type
    Str,
    /// Boolean (yes/no) question type
    Bool,
    /// JSON structured input type
    Json,
    /// YAML structured input type
    Yaml,
}
#[derive(Debug, Deserialize)]
pub struct Secret {
    /// Whether the secret should have confirmation
    #[serde(default)]
    pub confirm: bool,
    #[serde(default)]
    pub mistmatch_err: String,
}

#[derive(Debug, Deserialize)]
pub struct Validation {
    #[serde(default)]
    pub condition: String,
    #[serde(default = "get_default_error_message")]
    pub error_message: String,
}

fn get_default_condition() -> String {
    "true".to_string()
}

fn get_default_error_message() -> String {
    "Invalid answer".to_string()
}

/// Represents a single question in the configuration
#[derive(Debug, Deserialize)]
pub struct Question {
    /// Help text/prompt to display to the user
    #[serde(default)]
    pub help: String,
    /// Type of the question (string or boolean)
    #[serde(rename = "type")]
    pub r#type: Type,
    /// Optional default value for the question
    #[serde(default)]
    pub default: serde_json::Value,
    /// Available choices for string questions
    #[serde(default)]
    pub choices: Vec<String>,
    /// Available option for string questions
    #[serde(default)]
    pub multiselect: bool,
    /// Whether the string is a secret
    #[serde(default)]
    pub secret: Option<Secret>,
    #[serde(default)]
    pub ask_if: String,
    /// JSON Schema for validation (for Json and Yaml types)
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default = "get_default_validation")]
    pub validation: Validation,
}
/// Main configuration structure holding all questions
#[derive(Debug, Deserialize)]
pub struct ConfigV1 {
    #[serde(default = "get_default_template_suffix")]
    pub template_suffix: String,
    #[serde(default = "get_default_template_globs")]
    pub template_globs: Vec<String>,
    #[serde(default)]
    pub questions: IndexMap<String, Question>,
    #[serde(default = "get_default_post_hook_filename")]
    pub post_hook_filename: String,
    #[serde(default = "get_default_pre_hook_filename")]
    pub pre_hook_filename: String,
}

impl ConfigV1 {
    pub fn validate(&self) -> Result<(), Error> {
        if self.template_suffix.is_empty() {
            return Err(Error::ConfigValidation(
                "template_suffix must not be empty".into(),
            ));
        }
        if !self.template_suffix.starts_with('.') || self.template_suffix.len() < 2 {
            return Err(Error::ConfigValidation("template_suffix must start with '.' and have at least 1 character after it".into()));
        }
        Ok(())
    }
}

fn get_default_template_suffix() -> String {
    ".baker.j2".to_string()
}

fn get_default_template_globs() -> Vec<String> {
    Vec::new()
}

fn get_default_post_hook_filename() -> String {
    "post".to_string()
}

fn get_default_pre_hook_filename() -> String {
    "pre".to_string()
}

fn get_default_validation() -> Validation {
    Validation {
        condition: get_default_condition(),
        error_message: get_default_error_message(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "schemaVersion")]
pub enum Config {
    #[serde(rename = "v1")]
    V1(ConfigV1),
}

impl Config {
    pub fn load_config<P: AsRef<Path>>(template_root: P) -> Result<Self> {
        let template_root = template_root.as_ref().to_path_buf();
        let template_dir = path_to_str(&template_root)?.to_string();

        for config_file_name in CONFIG_LIST.iter() {
            let config_file_path = template_root.join(config_file_name);

            if config_file_path.exists() {
                let content = std::fs::read_to_string(config_file_path)?;
                let config: Config = match *config_file_name {
                    "baker.json" => serde_json::from_str(&content)?,
                    "baker.yaml" | "baker.yml" => serde_yaml::from_str(&content)?,
                    _ => unreachable!(),
                };

                return Ok(config);
            }
        }

        Err(Error::ConfigNotFound { template_dir, config_files: CONFIG_LIST.join(", ") })
    }
}

#[derive(Debug, PartialEq)]
pub enum QuestionType {
    MultipleChoice,
    SingleChoice,
    Text,
    Boolean,
    Json,
    Yaml,
}

#[derive(Debug)]
pub struct QuestionRendered {
    pub ask_if: bool,
    pub default: serde_json::Value,
    pub help: String,
    pub r#type: QuestionType,
}

pub trait IntoQuestionType {
    #[allow(clippy::wrong_self_convention)]
    fn into_question_type(&self) -> QuestionType;
}

impl IntoQuestionType for Question {
    fn into_question_type(&self) -> QuestionType {
        match (&self.r#type, self.choices.is_empty()) {
            (Type::Str, false) => {
                if self.multiselect {
                    QuestionType::MultipleChoice
                } else {
                    QuestionType::SingleChoice
                }
            }
            (Type::Str, true) => QuestionType::Text,
            (Type::Bool, _) => QuestionType::Boolean,
            (Type::Json, _) => QuestionType::Json,
            (Type::Yaml, _) => QuestionType::Yaml,
        }
    }
}

impl Question {
    fn process_structured_default_value(
        &self,
        default: serde_json::Value,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
        question_type: &QuestionType,
    ) -> serde_json::Value {
        // If the default is already a JSON object or array, use it directly
        if default.is_object() || default.is_array() {
            default
        } else if let Some(default_str) = default.as_str() {
            // If it's a string, try to render it as a template first
            let rendered_str = engine
                .render(default_str, answers, Some("default_value"))
                .unwrap_or(default_str.to_string());

            // Parse the string based on the question type
            match question_type {
                QuestionType::Json => {
                    serde_json::from_str(&rendered_str).unwrap_or(serde_json::json!({}))
                }
                QuestionType::Yaml => {
                    serde_yaml::from_str(&rendered_str).unwrap_or(serde_json::json!({}))
                }
                _ => unreachable!(),
            }
        } else {
            // Fallback to empty object
            serde_json::json!({})
        }
    }

    pub fn render(
        &self,
        question_key: &str,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
    ) -> QuestionRendered {
        // Renders default.
        let default = if let Some(answer) = answers.get(question_key) {
            // If answer in pre-filled answers we just return them as it is.
            answer.to_owned()
        } else {
            let default = self.default.clone();
            match self.into_question_type() {
                QuestionType::MultipleChoice => default,
                QuestionType::Boolean => {
                    let val = default.as_bool().unwrap_or(false);
                    serde_json::Value::Bool(val)
                }
                QuestionType::SingleChoice | QuestionType::Text => {
                    // Trying to extract str from default which is serde_json::Value,
                    // otherwise it return empty slice.
                    let default_str = default.as_str().unwrap_or_default();

                    // Trying to render given string.
                    // Otherwise returns an empty string.
                    let default_rendered = engine
                        .render(default_str, answers, Some("default_value"))
                        .unwrap_or_default();
                    serde_json::Value::String(default_rendered)
                }
                QuestionType::Json | QuestionType::Yaml => self
                    .process_structured_default_value(
                        default,
                        answers,
                        engine,
                        &self.into_question_type(),
                    ),
            }
        };

        // Sometimes "help" contain the value with the template strings.
        // This function renders it and returns rendered value.
        let help =
            engine.render(&self.help, answers, Some("help")).unwrap_or(self.help.clone());

        let ask_if = engine.execute_expression(&self.ask_if, answers).unwrap_or(true);

        QuestionRendered { default, ask_if, help, r#type: self.into_question_type() }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::template::get_template_engine;

    #[test]
    fn it_works_1() {
        let question = Question {
            help: "Hello, {{prev_answer}}".to_string(),
            r#type: Type::Bool,
            default: serde_json::Value::Null,
            ask_if: r#"prev_answer == "TEST""#.to_string(),
            secret: None,
            multiselect: false,
            choices: vec![],
            schema: None,
            validation: get_default_validation(),
        };
        let engine = get_template_engine();

        let answers = json!({
            "prev_answer": "World"
        });

        let result = question.render("question1".as_ref(), &answers, &engine);
        let QuestionRendered { ask_if, help, default, r#type } = result;
        assert!(!ask_if);
        assert_eq!(help, "Hello, World".to_string());
        assert_eq!(default, serde_json::Value::Bool(false));
        assert_eq!(r#type, QuestionType::Boolean);
    }

    #[test]
    fn it_works_2() {
        let question = Question {
            help: "{{question}}".to_string(),
            r#type: Type::Str,
            default: json!(vec!["Python".to_string(), "Django".to_string()]),
            ask_if: "".to_string(),
            secret: None,
            multiselect: true,
            choices: vec![
                "Python".to_string(),
                "Django".to_string(),
                "FastAPI".to_string(),
                "Next.JS".to_string(),
                "TypeScript".to_string(),
            ],
            schema: None,
            validation: get_default_validation(),
        };
        let engine = get_template_engine();

        let answers = json!({
            "question": "Please select your stack"
        });

        let result = question.render("question1".as_ref(), &answers, &engine);
        let QuestionRendered { ask_if, help, default, r#type } = result;
        assert!(ask_if);
        assert_eq!(help, "Please select your stack".to_string());
        assert_eq!(default, json!(vec!["Python".to_string(), "Django".to_string()]));
        assert_eq!(r#type, QuestionType::MultipleChoice);
    }

    #[test]
    fn it_works_3() {
        let question = Question {
            help: "".to_string(),
            r#type: Type::Str,
            default: serde_json::Value::Null,
            ask_if: "answer is not defined".to_string(),
            secret: None,
            multiselect: false,
            choices: vec![],
            schema: None,
            validation: get_default_validation(),
        };
        let engine = get_template_engine();

        let answers = json!({});

        let result = question.render("question1".as_ref(), &answers, &engine);
        let QuestionRendered { ask_if, r#type, .. } = result;
        assert!(ask_if);
        assert_eq!(r#type, QuestionType::Text);
    }
    #[test]
    fn it_works_4() {
        let question = Question {
            help: "".to_string(),
            r#type: Type::Str,
            default: serde_json::Value::Null,
            ask_if: "answer is not defined".to_string(),
            secret: None,
            multiselect: false,
            choices: vec![],
            schema: None,
            validation: get_default_validation(),
        };
        let engine = get_template_engine();

        let answers = json!({"answer": "Here is an answer"});

        let result = question.render("question1".as_ref(), &answers, &engine);
        let QuestionRendered { ask_if, r#type, .. } = result;
        assert!(!ask_if);
        assert_eq!(r#type, QuestionType::Text);
    }
    #[test]
    fn it_works_5() {
        let question = Question {
            help: "".to_string(),
            r#type: Type::Str,
            default: json!("This is a default value"),
            ask_if: "question1 is not defined".to_string(),
            secret: None,
            multiselect: false,
            choices: vec![],
            schema: None,
            validation: get_default_validation(),
        };
        let engine = get_template_engine();

        let answers = json!({"question1": "This is a default value for the question1"});

        let result = question.render("question1".as_ref(), &answers, &engine);
        let QuestionRendered { ask_if, r#type, default, .. } = result;
        assert!(!ask_if);
        assert_eq!(r#type, QuestionType::Text);
        assert_eq!(default, json!("This is a default value for the question1"));
    }
    #[test]
    fn it_works_6() {
        let question = Question {
            help: "".to_string(),
            r#type: Type::Str,
            default: json!("This is a default value"),
            ask_if: "question1 is not defined".to_string(),
            secret: None,
            multiselect: false,
            choices: vec![],
            schema: None,
            validation: get_default_validation(),
        };
        let engine = get_template_engine();

        let answers = json!({});

        let result = question.render("question1".as_ref(), &answers, &engine);
        let QuestionRendered { ask_if, r#type, default, .. } = result;
        assert!(ask_if);
        assert_eq!(r#type, QuestionType::Text);
        assert_eq!(default, json!("This is a default value"));
    }
}
