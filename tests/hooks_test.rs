use baker::hooks::{confirm_hooks_execution, get_hooks, Output};
use tempfile::TempDir;

#[test]
fn test_get_hooks() {
    let temp_dir = TempDir::new().unwrap();
    let (pre_hook, post_hook) = get_hooks(temp_dir.path());

    assert_eq!(pre_hook, temp_dir.path().join("hooks/pre_gen_project"));
    assert_eq!(post_hook, temp_dir.path().join("hooks/post_gen_project"));
}

#[test]
fn test_confirm_hooks_execution() {
    // Test with skip_hooks_check = true
    assert!(confirm_hooks_execution(true).unwrap());

    // Note: Testing user input would require mock stdin
}

#[test]
fn test_output_serialization() {
    let output = Output {
        template_dir: "/path/to/template",
        output_dir: "/path/to/output",
        context: &serde_json::json!({"key": "value"}),
    };

    let serialized = serde_json::to_string(&output).unwrap();
    assert!(serialized.contains("template_dir"));
    assert!(serialized.contains("output_dir"));
    assert!(serialized.contains("context"));
}
