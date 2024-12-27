use crate::config::{Question, QuestionType};
use crate::error::{Error, Result};
use crate::renderer::TemplateRenderer;

pub struct RenderedQuestion {
    pub ask_if: bool,
    pub default: serde_json::Value,
    pub help: Option<String>,
}

pub trait QuestionRenderer<'a> {
    fn render(
        &self,
        engine: &'a dyn TemplateRenderer,
        answers: serde_json::Value,
    ) -> RenderedQuestion;
    fn get_single_choice_default(&self) -> serde_json::Value;
    fn get_multiple_choice_default(&self) -> serde_json::Value;
    fn get_text_default(
        &self,
        engine: &'a dyn TemplateRenderer,
        current_context: &serde_json::Value,
    ) -> serde_json::Value;
    fn get_yes_no_default(&self) -> serde_json::Value;
}

pub fn read_from(
    mut reader: impl std::io::Read,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf).map_err(Error::IoError)?;

    let value = serde_json::from_str(&buf)
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    match value {
        serde_json::Value::Object(map) => Ok(map),
        _ => Ok(serde_json::Map::new()),
    }
}

impl<'a> QuestionRenderer<'a> for Question {
    fn render(
        &self,
        engine: &'a dyn TemplateRenderer,
        answers: serde_json::Value,
    ) -> RenderedQuestion {
        let default = match self.question_type() {
            QuestionType::MultipleChoice => self.get_multiple_choice_default(),
            QuestionType::SingleChoice => self.get_single_choice_default(),
            QuestionType::Text => self.get_text_default(engine, &answers),
            QuestionType::Boolean => self.get_yes_no_default(),
        };

        // Sometimes "help" contain the value with the template strings.
        // This function renders it and returns rendered value.
        let help = engine.render(&self.help, &answers).unwrap_or(self.help.clone());

        let ask = engine.execute_expression(&self.ask_if, &answers).unwrap_or(true);

        RenderedQuestion { default, ask_if: ask, help: Some(help) }
    }

    /// Retrieves the default value of single choice
    fn get_single_choice_default(&self) -> serde_json::Value {
        let default_value = if let Some(default_value) = &self.default {
            if let Some(default_str) = default_value.as_str() {
                self.choices.iter().position(|choice| choice == default_str).unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        serde_json::Value::Number(default_value.into())
    }

    fn get_multiple_choice_default(&self) -> serde_json::Value {
        let default_value = self
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

        let defaults_map: Vec<bool> = self
            .choices
            .iter()
            .map(|choice| default_value.contains_key(choice))
            .collect();

        serde_json::to_value(defaults_map).unwrap()
    }

    fn get_text_default(
        &self,
        engine: &'a dyn TemplateRenderer,
        current_context: &serde_json::Value,
    ) -> serde_json::Value {
        let default_value = if let Some(default_value) = &self.default {
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

    fn get_yes_no_default(&self) -> serde_json::Value {
        let default_value = if let Some(default_value) = &self.default {
            default_value.as_bool().unwrap_or(false)
        } else {
            false
        };

        serde_json::Value::Bool(default_value)
    }
}
