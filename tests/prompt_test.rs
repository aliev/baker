use std::io::Cursor;

#[cfg(test)]
mod tests {
    use crate::MockStdin;
    use baker::config::ConfigValue;
    use baker::prompt::{prompt_config_values, yes_no_prompt};
    use indexmap::IndexMap;

    #[test]
    fn test_yes_no_prompt() {
        assert_eq!(yes_no_prompt("yes"), (true, true));
        assert_eq!(yes_no_prompt("y"), (true, true));
        assert_eq!(yes_no_prompt("no"), (true, false));
        assert_eq!(yes_no_prompt("n"), (true, false));
        assert_eq!(yes_no_prompt("invalid"), (false, false));
    }

    #[test]
    #[ignore = "Requires interactive input"]
    fn test_prompt_config_values() {
        let mut config = IndexMap::new();
        config.insert(
            "test_string".to_string(),
            ConfigValue::String {
                question: "Enter test string".to_string(),
                default: "default".to_string(),
            },
        );
        config.insert(
            "test_array".to_string(),
            ConfigValue::Array {
                question: "Select an option".to_string(),
                choices: vec!["option1".to_string(), "option2".to_string()],
            },
        );

        // Simulate user input
        let mock_input = "test_value\n1\n";
        let _mock_stdin = MockStdin::new(mock_input);

        let result = prompt_config_values(config);
        assert!(result.is_ok());
    }
}

// Mock stdin for testing
struct MockStdin {
    cursor: Cursor<Vec<u8>>,
}

impl MockStdin {
    fn new(input: &str) -> Self {
        Self {
            cursor: Cursor::new(input.as_bytes().to_vec()),
        }
    }
}

impl std::io::Read for MockStdin {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cursor.read(buf)
    }
}
