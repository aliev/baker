use indexmap::IndexMap;
use log::debug;
use std::io::{self, Write};

use crate::error::{BakerError, BakerResult};

fn yes_no_prompt(value: &str) -> (bool, bool) {
    let yes_choices = ["1", "true", "t", "yes", "y", "on"];
    let no_choices = ["0", "false", "f", "no", "n", "off"];

    let is_yes_choice = yes_choices.iter().any(|&i| i == value);
    let is_no_choice = no_choices.iter().any(|&i| i == value);

    (
        is_yes_choice || is_no_choice,
        is_yes_choice && !is_no_choice,
    )
}

pub fn read_input() -> BakerResult<String> {
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn prompt_config_values(
    config: IndexMap<String, serde_json::Value>,
) -> BakerResult<serde_json::Value> {
    debug!("Starting interactive configuration...");
    let mut final_context = config.clone();
    // Use iter_mut() to maintain the original order from the IndexMap
    for (key, value) in final_context.iter_mut() {
        match value {
            serde_json::Value::String(default_value) => {
                print!("{} [{}]: ", key.replace("_", " "), default_value);

                let input = read_input()?;

                let (is_yes_no_choice, yes_no_choice) = yes_no_prompt(if input.is_empty() {
                    default_value
                } else {
                    &input
                });

                if is_yes_no_choice {
                    *value = serde_json::Value::Bool(yes_no_choice);
                } else if !input.is_empty() {
                    *value = serde_json::Value::String(input);
                }
            }
            serde_json::Value::Array(arr) => {
                println!("Select {}:", key.replace("_", " "));
                for (i, opt) in arr.iter().enumerate() {
                    println!("{} - {}", i + 1, opt.as_str().unwrap_or_default());
                }
                print!("Choose from 1-{} [1]: ", arr.len());
                let input = read_input()?;
                let choice = if input.is_empty() {
                    0
                } else {
                    input.parse::<usize>().unwrap_or(1).saturating_sub(1)
                };
                if choice < arr.len() {
                    *value = arr[choice].clone();
                }
            }
            _ => {}
        }
    }

    let context =
        serde_json::to_value(final_context).map_err(|e| BakerError::ConfigError(e.to_string()))?;
    debug!("Final configuration: {:#?}", context);
    Ok(context)
}
