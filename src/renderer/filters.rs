use log::warn;
use regex::Regex;

// Re-export all the case conversion and string manipulation functions
pub use cruet::{
    case::{
        camel::to_camel_case, kebab::to_kebab_case, pascal::to_pascal_case,
        screaming_snake::to_screaming_snake_case, snake::to_snake_case,
        table::to_table_case, train::to_train_case,
    },
    string::{pluralize::to_plural, singularize::to_singular},
    suffix::foreign_key::to_foreign_key,
};

/// Custom regex filter for template processing.
///
/// Tests if a string matches a given regular expression pattern.
///
/// # Arguments
/// * `val` - The string to test
/// * `re` - The regular expression pattern
///
/// # Returns
/// * `bool` - True if the string matches the pattern, false otherwise
pub fn regex_filter(val: &str, re: &str) -> bool {
    match Regex::new(re) {
        Ok(re) => re.is_match(val),
        Err(err) => {
            warn!("Invalid regex '{re}': {err}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_filter_matches() {
        assert!(regex_filter("hello123", r"hello\d+"));
    }

    #[test]
    fn test_regex_filter_no_match() {
        assert!(!regex_filter("hello", r"\d+"));
    }

    #[test]
    fn test_regex_filter_invalid_regex() {
        assert!(!regex_filter("anything", r"([unclosed"));
    }
}
