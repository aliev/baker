use baker::cli::{run, Args, SkipConfirm::All};
use test_log::test;
mod utils;
use utils::run_and_assert;

#[test]
fn test_single_choice_question() {
    run_and_assert(
        "tests/templates/single_choice",
        "tests/expected/single_choice",
        Some(r#"{"favourite_language": "Go"}"#),
    );
}

#[test]
fn test_multiple_choice_question() {
    run_and_assert(
        "tests/templates/multiple_choice",
        "tests/expected/multiple_choice",
        Some(r#"{"favourite_languages": ["Go", "TypeScript"]}"#),
    );
}

#[test]
fn test_yaml_complex_type() {
    let yaml_config = r#"
    {
        "environments": {
            "development": {
                "url": "http://localhost:8000",
                "debug": true
            },
            "production": {
                "url": "https://staging.example.com",
                "debug": false
            }
        }
    }"#;
    run_and_assert(
        "tests/templates/yaml_type",
        "tests/expected/yaml_type",
        Some(yaml_config),
    );
}

#[test]
fn test_conditional_questions_python() {
    run_and_assert(
        "tests/templates/conditional_questions",
        "tests/expected/conditional_questions_python",
        Some(
            r#"{"language": "Python", "py_framework": "Django", "include_docker": true}"#,
        ),
    );
}

#[test]
fn test_conditional_questions_rust() {
    run_and_assert(
        "tests/templates/conditional_questions",
        "tests/expected/conditional_questions_rust",
        Some(r#"{"language": "Rust"}"#),
    );
}

#[test]
fn test_validation_patterns() {
    run_and_assert(
        "tests/templates/validation",
        "tests/expected/validation",
        Some(
            r#"{"project_name": "My Test Project", "age": "25", "email": "test@example.com"}"#,
        ),
    );
}

#[test]
fn test_builtin_filters() {
    run_and_assert(
        "tests/templates/builtin_filters",
        "tests/expected/builtin_filters",
        Some(
            r#"{"project_name": "Example Project", "project_slug": "example_project", "class_name": "ExampleProject", "table_name": "example_projects", "constant_name": "EXAMPLE_PROJECT"}"#,
        ),
    );
}

#[test]
fn test_templated_filenames_with_database() {
    run_and_assert(
        "tests/templates/templated_filenames",
        "tests/expected/templated_filenames",
        Some(
            r#"{"project_name": "my awesome project", "project_slug": "my_awesome_project", "project_class": "MyAwesomeProject", "use_database": true}"#,
        ),
    );
}

#[test]
fn test_templated_filenames_without_database() {
    run_and_assert(
        "tests/templates/templated_filenames",
        "tests/expected/templated_filenames_no_db",
        Some(
            r#"{"project_name": "my awesome project", "project_slug": "my_awesome_project", "project_class": "MyAwesomeProject", "use_database": false}"#,
        ),
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_custom_hooks() {
    run_and_assert(
        "tests/templates/custom_hooks",
        "tests/expected/custom_hooks",
        Some(r#"{"username": "testuser", "project_type": "web"}"#),
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_hook_runner_unix() {
    run_and_assert(
        "tests/templates/hook_runner_unix",
        "tests/expected/hook_runner_unix",
        Some(r#"{"python_version": "3"}"#),
    );
}

#[test]
#[cfg(target_os = "macos")]
fn test_platform_variables() {
    run_and_assert(
        "tests/templates/platform_variables",
        "tests/expected/platform_variables",
        Some(r#"{"project_name": "cross-platform-app"}"#),
    );
}

#[test]
#[cfg(target_os = "windows")]
fn test_hook_runner_windows() {
    run_and_assert(
        "tests/templates/hook_runner_windows",
        "tests/expected/hook_runner_windows",
        None,
    );
}

#[test]
fn test_non_interactive_mode_with_defaults() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/builtin_filters".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: 2,
        answers: None, // Test default values being used
        skip_confirms: vec![All],
        non_interactive: true,
        dry_run: false,
    };
    run(args).unwrap();

    // Check that the file was created with default values
    let output_file = tmp_dir.path().join("output.md");
    assert!(output_file.exists());
    let content = std::fs::read_to_string(output_file).unwrap();
    assert!(content.contains("Example Project"));
}

#[test]
fn test_template_with_different_suffix() {
    run_and_assert(
        "tests/templates/different_template_suffix",
        "tests/expected/different_template_suffix",
        None,
    );
}

#[test]
fn test_nested_answer_context() {
    // Test that previous answers are available in later questions
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "examples/demo".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: 2,
        answers: Some(r#"{"project_name": "Test Project", "project_author": "Test Author", "project_slug": "test_project", "use_tests": true}"#.to_string()),
        skip_confirms: vec![All],
        non_interactive: true,
        dry_run: false,
    };
    run(args).unwrap();

    // Verify the content uses interpolated values correctly
    let readme_file = tmp_dir.path().join("CONTRIBUTING.md");
    assert!(readme_file.exists());
    let content = std::fs::read_to_string(readme_file).unwrap();
    assert!(content.contains("Test Project"));
    assert!(content.contains("Test Author"));
}

#[test]
fn test_jsonschema_file() {
    run_and_assert(
        "tests/templates/jsonschema_file",
        "tests/expected/jsonschema_file",
        Some(
            r#"{"database_config": {"engine": "mysql", "host": "db.example.com", "port": 3306, "ssl": true}}"#,
        ),
    );
}

#[test]
#[should_panic(expected = "JSON Schema validation error")]
fn test_jsonschema_file_invalid_engine() {
    run_and_assert(
        "tests/templates/jsonschema_file",
        "tests/expected/jsonschema_file",
        Some(
            r#"{"database_config": {"engine": "oracle", "host": "db.example.com", "port": 3306}}"#,
        ),
    );
}

#[test]
#[should_panic(expected = "JSON Schema validation error")]
fn test_jsonschema_file_missing_required_field() {
    run_and_assert(
        "tests/templates/jsonschema_file",
        "tests/expected/jsonschema_file",
        Some(r#"{"database_config": {"engine": "mysql", "host": "db.example.com"}}"#),
    );
}

#[test]
#[should_panic(expected = "JSON Schema validation error")]
fn test_jsonschema_file_invalid_port_type() {
    run_and_assert(
        "tests/templates/jsonschema_file",
        "tests/expected/jsonschema_file",
        Some(
            r#"{"database_config": {"engine": "mysql", "host": "db.example.com", "port": "3306"}}"#,
        ),
    );
}

#[test]
#[should_panic(expected = "JSON Schema validation error")]
fn test_jsonschema_file_invalid_port_range() {
    run_and_assert(
        "tests/templates/jsonschema_file",
        "tests/expected/jsonschema_file",
        Some(
            r#"{"database_config": {"engine": "mysql", "host": "db.example.com", "port": 99999}}"#,
        ),
    );
}

#[test]
fn test_import_root() {
    run_and_assert(
        "examples/import_root",
        "tests/expected/import_root",
        Some(r#"{"project_name": "my-project"}"#),
    );
}
