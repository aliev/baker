mod utils;
use utils::run_and_assert;

#[cfg(test)]
mod tests {
    use crate::run_and_assert;

    #[test]
    fn test_demo_copy() {
        run_and_assert(
            "examples/demo",
            "tests/expected/demo",
            Some("{\"project_name\": \"demo\", \"project_author\": \"demo\", \"project_slug\": \"demo\", \"use_tests\": true}")
        );
    }

    #[test]
    fn test_demo_copy_use_tests_false() {
        run_and_assert(
            "examples/demo",
            "tests/expected/demo_tests_false",
            Some("{\"project_name\": \"demo\", \"project_author\": \"demo\", \"project_slug\": \"demo\", \"use_tests\": false}")
        );
    }

    #[test]
    fn test_filters_example() {
        run_and_assert(
            "examples/filters",
            "tests/expected/filters",
            Some("{\"project_name\": \"project name is filters\"}"),
        );
    }

    #[test]
    fn test_jsonschema_default() {
        run_and_assert(
            "tests/templates/jsonschema",
            "tests/expected/jsonschema-default",
            None,
        );
    }

    #[test]
    fn test_jsonschema() {
        run_and_assert(
            "tests/templates/jsonschema",
            "tests/expected/jsonschema",
            Some("{\"database_config\":{\"engine\":\"redis\",\"host\":\"localhost\",\"port\":6379}}")
        );
    }

    #[test]
    fn test_import() {
        run_and_assert(
            "examples/import",
            "tests/expected/import",
            Some("{\"database_config\":{\"engine\":\"redis\",\"host\":\"localhost\",\"port\":6379}}")
        );
    }

    #[test]
    fn test_import_directory() {
        run_and_assert(
            "examples/import_directory",
            "tests/expected/import_directory",
            None,
        );
    }

    #[test]
    fn test_different_template_suffix() {
        run_and_assert(
            "tests/templates/different_template_suffix",
            "tests/expected/different_template_suffix",
            None,
        );
    }

    #[test]
    #[should_panic(
        expected = "called `Result::unwrap()` on an `Err` value: ConfigValidation(\"template_suffix must start with '.' and have at least 1 character after it\")"
    )]
    fn test_wrong_template_suffix() {
        run_and_assert("tests/templates/wrong_template_suffix", "", None);
    }

    #[test]
    #[should_panic(
        expected = "called `Result::unwrap()` on an `Err` value: ConfigValidation(\"template_suffix must not be empty\")"
    )]
    fn test_empty_template_suffix() {
        run_and_assert("tests/templates/empty_template_suffix", "", None);
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_pre_hook_cli_merge() {
        run_and_assert(
            "tests/templates/pre_hook_merge",
            "tests/expected/pre_hook_merge",
            Some("{\"username\":\"cliuser\"}"),
        );
    }
}
