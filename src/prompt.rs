//! User input handling and prompt functionality for Baker.
//! This module manages interactive configuration prompts and user input validation
//! for template variables and configuration values.
use indexmap::IndexMap;
use std::io::{self, Write};

use crate::config::ConfigValue;
use crate::error::BakerResult;

/// Evaluates a string input as a yes/no response.
///
/// # Arguments
/// * `value` - The string to evaluate
///
/// # Returns
/// * `(bool, bool)` - Tuple containing:
///   - Whether the input was valid (true/false)
///   - Whether the input was affirmative (yes)
///
/// # Examples
/// ```
/// use baker::prompt::yes_no_prompt;
/// assert_eq!(yes_no_prompt("yes"), (true, true));
/// assert_eq!(yes_no_prompt("no"), (true, false));
/// assert_eq!(yes_no_prompt("invalid"), (false, false));
/// ```
pub fn yes_no_prompt(value: &str) -> (bool, bool) {
    let yes_choices = ["1", "true", "t", "yes", "y", "on"];
    let no_choices = ["0", "false", "f", "no", "n", "off"];

    let is_yes_choice = yes_choices.iter().any(|&i| i == value);
    let is_no_choice = no_choices.iter().any(|&i| i == value);

    (
        is_yes_choice || is_no_choice,
        is_yes_choice && !is_no_choice,
    )
}

/// Reads a line of input from stdin.
///
/// # Returns
/// * `BakerResult<String>` - The trimmed input string
///
/// # Errors
/// * Returns `BakerError::IoError` if stdin read fails
pub fn read_input() -> BakerResult<String> {
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Prompts for and processes configuration values interactively.
///
/// # Arguments
/// * `config` - Initial configuration key-value pairs
///
/// # Returns
/// * `BakerResult<serde_json::Value>` - Processed configuration with user inputs
///
/// # Notes
/// - Maintains order of configuration keys using IndexMap
/// - Handles different value types (strings, arrays, booleans)
/// - Supports default values and validation
///
/// # Example
/// ```
/// use indexmap::IndexMap;
/// use baker::config::ConfigValue;
/// use baker::prompt::prompt_config_values;
///
/// # fn main() -> baker::error::BakerResult<()> {
/// let mut config = IndexMap::new();
/// config.insert(
///     "project_name".to_string(),
///     ConfigValue::String {
///         question: "Project name?".to_string(),
///         default: "My Project".to_string(),
///     }
/// );
/// let processed = prompt_config_values(config)?;
/// # Ok(())
/// # }
///
pub fn prompt_config_values(
    config: IndexMap<String, ConfigValue>,
) -> BakerResult<serde_json::Value> {
    let mut final_context = serde_json::Map::new();

    for (key, value) in config {
        match value {
            ConfigValue::String { question, default } => {
                print!("{} [{}]: ", question, default);
                let input = read_input()?;
                final_context.insert(
                    key,
                    serde_json::Value::String(if input.is_empty() { default } else { input }),
                );
            }
            ConfigValue::Boolean { question, default } => {
                print!("{} [{}]: ", question, if default { "Y/n" } else { "y/N" });
                let input = read_input()?;
                let (_, choice) = yes_no_prompt(if input.is_empty() {
                    if default {
                        "yes"
                    } else {
                        "no"
                    }
                } else {
                    &input
                });
                final_context.insert(key, serde_json::Value::Bool(choice));
            }
            ConfigValue::Array { question, choices } => {
                println!("{}:", question);
                for (i, choice) in choices.iter().enumerate() {
                    println!("{} - {}", i + 1, choice);
                }
                print!("Choose from 1-{} [1]: ", choices.len());
                let input = read_input()?;
                let choice = if input.is_empty() {
                    0
                } else {
                    input.parse::<usize>().unwrap_or(1).saturating_sub(1)
                };
                if choice < choices.len() {
                    final_context.insert(key, serde_json::Value::String(choices[choice].clone()));
                }
            }
        }
    }

    Ok(serde_json::Value::Object(final_context))
}
