use crate::{
    question::{Question, QuestionType, WithQuestionType},
    renderer::TemplateRenderer,
};

pub struct QuestionRendered {
    pub ask_if: bool,
    pub default: serde_json::Value,
    pub help: Option<String>,
}

pub trait QuestionRenderer<'a> {
    fn get_default(
        &self,
        question: &Question,
        answers: &serde_json::Value,
        engine: &'a dyn TemplateRenderer,
    ) -> serde_json::Value {
        match question.question_type() {
            QuestionType::MultipleChoice => self.get_multiple_choice_default(question),
            QuestionType::SingleChoice => self.get_single_choice_default(question),
            QuestionType::Text => self.get_text_default(question, answers, engine),
            QuestionType::Boolean => self.get_yes_no_default(question),
        }
    }

    fn render(
        &self,
        question: &Question,
        answers: &serde_json::Value,
        engine: &'a dyn TemplateRenderer,
    ) -> QuestionRendered {
        let default = self.get_default(question, answers, engine);

        // Sometimes "help" contain the value with the template strings.
        // This function renders it and returns rendered value.
        let help = Some(
            engine.render(&question.help, answers).unwrap_or(question.help.clone()),
        );

        let ask_if =
            engine.execute_expression(&question.ask_if, answers).unwrap_or(true);

        QuestionRendered { default, ask_if, help }
    }

    /// Retrieves the default value of single choice
    fn get_single_choice_default(&self, question: &Question) -> serde_json::Value {
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
        answers: &serde_json::Value,
        engine: &'a dyn TemplateRenderer,
    ) -> serde_json::Value {
        let default_value = if let Some(default_value) = &question.default {
            if let Some(s) = default_value.as_str() {
                engine.render(s, answers).unwrap_or_default()
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

impl QuestionRenderer<'_> for Question {}
