use baker::{
    error::BakerError,
    parser::{get_default_answers, get_value_or_default},
};
use serde_json::json;

#[test]
fn test_value_exists_in_parsed_context() {
    let key = "key1";
    let parsed = json!({
        "key1": "value1",
        "key2": "value2"
    });
    let default_value = json!("default_value");

    let result = get_value_or_default(key, parsed, default_value).unwrap();

    assert_eq!(result, (key.to_string(), json!("value1")));
}

#[test]
fn test_value_missing_in_parsed_context_but_default_exists() {
    let key = "key3";
    let parsed = json!({
        "key1": "value1",
        "key2": "value2"
    });
    let default_value = json!("default_value");

    let result = get_value_or_default(key, parsed, default_value).unwrap();

    assert_eq!(result, (key.to_string(), json!("default_value")));
}

#[test]
fn test_value_missing_in_both_parsed_context_and_default() {
    let key = "key3".to_string();
    let parsed = json!({
        "key1": "value1",
        "key2": "value2"
    });
    let default_value = serde_json::Value::Null;

    let result = get_value_or_default(key.clone(), parsed, default_value).unwrap();

    assert_eq!(result, (key, serde_json::Value::Null));
}

#[test]
fn test_value_is_null_in_parsed_context() {
    let key = "key1".to_string();
    let parsed = json!({
        "key1": null,
        "key2": "value2"
    });
    let default_value = json!("default_value");

    let result = get_value_or_default(key.clone(), parsed, default_value).unwrap();

    assert_eq!(result, (key, serde_json::Value::Null));
}

#[test]
fn test_empty_context() {
    let context = "";
    let result = get_default_answers(context).unwrap();
    assert_eq!(result, serde_json::Value::Null);
}

#[test]
fn test_valid_json_context() {
    let context = r#"{"key": "value"}"#;
    let result = get_default_answers(context).unwrap();
    assert_eq!(result, json!({"key": "value"}));
}

#[test]
fn test_invalid_json_context() {
    let context = r#"{"key": "value""#; // Missing closing brace
    let result = get_default_answers(context);
    assert!(result.is_err());
    if let Err(BakerError::TemplateError(err_msg)) = result {
        assert!(err_msg.contains("Failed to parse context as JSON"));
    } else {
        panic!("Expected BakerError::TemplateError");
    }
}

#[test]
fn test_context_with_whitespace() {
    let context = " ";
    let result = get_default_answers(context);
    assert!(result.is_err());
    if let Err(BakerError::TemplateError(err_msg)) = result {
        assert!(err_msg.contains("Failed to parse context as JSON"));
    } else {
        panic!("Expected BakerError::TemplateError");
    }
}

#[test]
fn test_numeric_json_context() {
    let context = "42";
    let result = get_default_answers(context).unwrap();
    assert_eq!(result, json!(42));
}
