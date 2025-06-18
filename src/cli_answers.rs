use crate::{
    config::{Config, QuestionRendered},
    dialoguer::ask_question,
    error::{Error, Result},
    ioutils::{parse_string_to_json, read_from},
    renderer::TemplateRenderer,
    validation::{validate_answer, ValidationError},
};
use serde_json::json;
use std::process::ChildStdout;

/// Handles the collection and processing of answers from various sources
pub struct AnswerProcessor<'a> {
    renderer: &'a dyn TemplateRenderer,
    non_interactive: bool,
}

impl<'a> AnswerProcessor<'a> {
    pub fn new(renderer: &'a dyn TemplateRenderer, non_interactive: bool) -> Self {
        Self {
            renderer,
            non_interactive,
        }
    }

    /// Retrieves initial answers from command line arguments or pre-hook output
    pub fn get_initial_answers(
        &self,
        answers_arg: Option<String>,
        pre_hook_stdout: Option<ChildStdout>,
    ) -> Result<serde_json::Map<String, serde_json::Value>> {
        if let Some(answers_arg) = answers_arg {
            // From command line argument
            let answers_str = if answers_arg == "-" {
                read_from(std::io::stdin())?
            } else {
                answers_arg
            };
            parse_string_to_json(answers_str)
        } else if let Some(pre_hook_stdout) = pre_hook_stdout {
            // Read and parse pre-hook output
            let result = read_from(pre_hook_stdout).unwrap_or_default();

            log::debug!(
                "Pre-hook stdout content (attempting to parse as JSON answers): {}",
                result
            );

            serde_json::from_str::<serde_json::Value>(&result).map_or_else(
                |e| {
                    log::warn!("Failed to parse hook output as JSON: {}", e);
                    Ok(serde_json::Map::new())
                },
                |value| match value {
                    serde_json::Value::Object(map) => Ok(map),
                    _ => Ok(serde_json::Map::new()),
                },
            )
        } else {
            Ok(serde_json::Map::new())
        }
    }

    /// Processes questions interactively and collects answers
    pub fn collect_answers(
        &self,
        config: &Config,
        mut answers: serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let Config::V1(config) = config;

        for (key, question) in &config.questions {
            loop {
                let QuestionRendered { help, default, ask_if, .. } =
                    question.render(key, &json!(answers), self.renderer);

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
                        answers.insert(key.clone(), default.clone());
                        break;
                    }
                }

                let answer = match ask_question(question, &default, help) {
                    Ok(answer) => answer,
                    Err(err) => match err {
                        Error::JSONParseError(_) | Error::YAMLParseError(_) => {
                            println!("{}", err);
                            continue;
                        }
                        _ => return Err(err),
                    },
                };

                answers.insert(key.clone(), answer.clone());
                let _answers = serde_json::Value::Object(answers.clone());

                match validate_answer(question, &answer, self.renderer, &_answers) {
                    Ok(_) => break,
                    Err(err) => match err {
                        ValidationError::JsonSchema(msg) => println!("{}", msg),
                        ValidationError::FieldValidation(msg) => println!("{}", msg),
                    },
                }
            }
        }

        Ok(serde_json::Value::Object(answers))
    }
}
