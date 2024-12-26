use crate::config::{Question, ValueType};
use crate::error::{Error, Result};
use crate::renderer::TemplateRenderer;

pub enum QuestionType {
    MultipleChoice,
    SingleChoice,
    Text,
    YesNo,
}

pub struct Action {
    pub prompt: bool,
    pub question_type: QuestionType,
    pub default_value: serde_json::Value,
    pub help_rendered: String,
}

pub struct AnswersParser<'a> {
    engine: &'a dyn TemplateRenderer,
    preloaded_answers: serde_json::Value,
}

impl<'a> AnswersParser<'a> {
    pub fn new(engine: &'a dyn TemplateRenderer) -> Self {
        Self { engine, preloaded_answers: serde_json::Value::Null }
    }

    pub fn read_from(&mut self, mut reader: impl std::io::Read) -> Result<()> {
        let mut buf = String::new();
        reader.read_to_string(&mut buf).map_err(Error::IoError)?;

        self.preloaded_answers =
            serde_json::from_str(&buf).unwrap_or(serde_json::Value::Null);

        Ok(())
    }

    pub fn parse(
        &self,
        key: &String,
        question: &Question,
        current_context: serde_json::Value,
    ) -> Action {
        let preloaded_answer = self.preloaded_answers.get(key);
        let (question_type, default_value) = match question.value_type {
            ValueType::Str => {
                if !question.choices.is_empty() {
                    if question.multiselect {
                        let default_value = self.get_multiple_choice_default(question);
                        (QuestionType::MultipleChoice, default_value)
                    } else {
                        // Extracts the default value from config.default (baker.yaml)
                        // if the value contains the template string it renders it.
                        let default_value = self.get_single_choice_default(question);
                        (QuestionType::SingleChoice, default_value)
                    }
                } else {
                    let default_value =
                        self.get_text_default(question, &current_context, self.engine);
                    (QuestionType::Text, default_value)
                }
            }
            ValueType::Bool => {
                let default_value = self.get_yes_no_default(question);
                (QuestionType::YesNo, default_value)
            }
        };

        if let Some(default_answer_value) = preloaded_answer {
            // Return the default answer
            return Action {
                default_value: default_answer_value.clone(),
                prompt: false,
                question_type,
                help_rendered: "".to_string(),
            };
        }
        // Sometimes "help" contain the value with the template strings.
        // This function renders it and returns rendered value.
        let help_rendered = self
            .engine
            .render(&question.help, &current_context)
            .unwrap_or(question.help.clone());

        let ask = self
            .engine
            .execute_expression(&question.ask_if, &current_context)
            .unwrap_or(true);

        Action { default_value, prompt: ask, question_type, help_rendered }
    }

    /// Retrieves the default value of single choice
    pub fn get_single_choice_default(&self, question: &Question) -> serde_json::Value {
        let default_value = if let Some(default_value) = &question.default {
            if let Some(default_str) = default_value.as_str() {
                question
                    .choices
                    .iter()
                    .position(|choice| choice == default_str)
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        serde_json::Value::Number(default_value.into())
    }

    fn get_multiple_choice_default(&self, question: &Question) -> serde_json::Value {
        let default_value = question
            .default
            .as_ref()
            .and_then(|default_value| {
                if let Some(default_obj) = default_value.as_object() {
                    Some(default_obj.clone())
                } else if let Some(default_arr) = default_value.as_array() {
                    let map = default_arr
                        .iter()
                        .filter_map(|value| {
                            value
                                .as_str()
                                .map(|s| (s.to_string(), serde_json::Value::Bool(true)))
                        })
                        .collect();
                    Some(map)
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let defaults_map: Vec<bool> = question
            .choices
            .iter()
            .map(|choice| default_value.contains_key(choice))
            .collect();

        serde_json::to_value(defaults_map).unwrap()
    }

    fn get_text_default(
        &self,
        question: &Question,
        current_context: &serde_json::Value,
        engine: &dyn TemplateRenderer,
    ) -> serde_json::Value {
        let default_value = if let Some(default_value) = &question.default {
            if let Some(s) = default_value.as_str() {
                engine.render(s, current_context).unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        serde_json::Value::String(default_value)
    }

    fn get_yes_no_default(&self, question: &Question) -> serde_json::Value {
        let default_value = if let Some(default_value) = &question.default {
            default_value.as_bool().unwrap_or(false)
        } else {
            false
        };

        serde_json::Value::Bool(default_value)
    }
}
