use crate::config::{Question, ValueType};
use crate::error::{Error, Result};
use crate::prompt::Prompter;
use crate::renderer::TemplateRenderer;
use indexmap::IndexMap;
use std::io::Read;
use std::process::ChildStdout;

pub enum QuestionType {
    MultipleChoice,
    SingleChoice,
    Text,
    YesNo,
}

#[derive(Debug)]
pub enum AnswerSource {
    Stdin,
    PreHookStdout(ChildStdout),
    None,
}

/// Retrieves the default value of single choice
pub fn get_single_choice_default(question: &Question) -> serde_json::Value {
    let default_value = if let Some(default_value) = &question.default {
        if let Some(default_str) = default_value.as_str() {
            question.choices.iter().position(|choice| choice == default_str).unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    };

    serde_json::Value::Number(default_value.into())
}

pub fn get_multiple_choice_default(question: &Question) -> serde_json::Value {
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

pub fn get_text_default(
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

pub fn get_yes_no_default(question: &Question) -> serde_json::Value {
    let default_value = if let Some(default_value) = &question.default {
        default_value.as_bool().unwrap_or(false)
    } else {
        false
    };

    serde_json::Value::Bool(default_value)
}

pub fn load_from_stdin() -> Result<serde_json::Value> {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer)?;
    let out = buffer.trim().to_string();
    Ok(serde_json::from_str(&out).unwrap_or(serde_json::Value::Null))
}

pub fn load_from_hook(mut stdout: ChildStdout) -> Result<serde_json::Value> {
    let mut buf = String::new();
    stdout.read_to_string(&mut buf).map_err(Error::IoError)?;
    Ok(serde_json::from_str(&buf).unwrap_or(serde_json::Value::Null))
}

pub fn get_answers_from(
    take_from_stdin: bool,
    pre_hook_stdout: Option<ChildStdout>,
) -> Result<serde_json::Value> {
    let answers_source = match (take_from_stdin, pre_hook_stdout) {
        (true, _) => AnswerSource::Stdin,
        (false, Some(stdout)) => AnswerSource::PreHookStdout(stdout),
        (false, None) => AnswerSource::None,
    };

    match answers_source {
        AnswerSource::Stdin => load_from_stdin(),
        AnswerSource::PreHookStdout(stdout) => load_from_hook(stdout),
        AnswerSource::None => Ok(serde_json::Value::Null),
    }
}

pub fn get_answers(
    engine: &dyn TemplateRenderer,
    prompt: &dyn Prompter,
    questions: IndexMap<String, Question>,
    preloaded_answers: serde_json::Value,
) -> Result<serde_json::Value> {
    let mut answers = serde_json::Map::new();

    for (key, question) in questions {
        let current_context = serde_json::Value::Object(answers.clone());

        let preloaded_answer = preloaded_answers.get(&key);

        let (question_type, default_value) = match question.value_type {
            ValueType::Str => {
                if !question.choices.is_empty() {
                    if question.multiselect {
                        let default_value = get_multiple_choice_default(&question);
                        (QuestionType::MultipleChoice, default_value)
                    } else {
                        // Extracts the default value from config.default (baker.yaml)
                        // if the value contains the template string it renders it.
                        let default_value = get_single_choice_default(&question);
                        (QuestionType::SingleChoice, default_value)
                    }
                } else {
                    let default_value =
                        get_text_default(&question, &current_context, engine);
                    (QuestionType::Text, default_value)
                }
            }
            ValueType::Bool => {
                let default_value = get_yes_no_default(&question);
                (QuestionType::YesNo, default_value)
            }
        };

        let value = if let Some(default_answer_value) = preloaded_answer {
            // Return the default answer
            default_answer_value.clone()
        } else {
            // Sometimes "help" contain the value with the template strings.
            // This function renders it and returns rendered value.
            let help_rendered = engine
                .render(&question.help, &current_context)
                .unwrap_or(question.help.clone());

            let when_rendered = engine
                .render(&question.when, &current_context)
                .unwrap_or(question.when.clone());

            let prompt_question: bool =
                serde_json::from_str(&when_rendered).unwrap_or(true);

            if prompt_question {
                prompt.answer(question_type, default_value, help_rendered, question)?
            } else {
                default_value
            }
        };
        answers.insert(key, value);
    }

    Ok(serde_json::Value::Object(answers))
}
