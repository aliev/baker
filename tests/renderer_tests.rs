#[cfg(test)]
mod tests {
    use baker::cli::{run, Args, SkipConfirm::All};
    use baker::renderer::{MiniJinjaRenderer, TemplateRenderer};
    use log::debug;
    use minijinja::functions::debug;
    use serde_json::json;
    use std::fs;
    use std::path::Path;
    use test_log::test;
    use walkdir::WalkDir;

    fn test_template(template: &str, expected: &str) {
        let renderer = MiniJinjaRenderer::new();
        let result = renderer.render(template, &json!({}), None).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_string_conversion_case_filters() {
        test_template("{{ 'hello world' | camel_case }}", "helloWorld");
        test_template("{{ 'hello world' | kebab_case }}", "hello-world");
        test_template("{{ 'hello world' | pascal_case }}", "HelloWorld");
        test_template("{{ 'hello world' | screaming_snake_case }}", "HELLO_WORLD");
        test_template("{{ 'hello world' | snake_case }}", "hello_world");
        test_template("{{ 'Hello World' | table_case }}", "hello_worlds");
        test_template("{{ 'hello world' | train_case }}", "Hello-World");
        test_template("{{ 'car' | plural }}", "cars");
        test_template("{{ 'cars' | singular }}", "car");
        test_template("{{ 'User' | foreign_key }}", "user_id");
        test_template("{{ 'Order Item' | foreign_key }}", "order_item_id");
        test_template("{{ 'orderItem' | foreign_key }}", "order_item_id");
        test_template("{{ 'OrderItem' | foreign_key }}", "order_item_id");
        test_template("{{ 'order_item' | foreign_key }}", "order_item_id");
        test_template("{{ 'order-item' | foreign_key }}", "order_item_id");
        test_template("{{ 'ORDER' | foreign_key }}", "order_id");
        test_template("{{ 'OrderITEM' | foreign_key }}", "order_item_id");
    }

    #[test]
    fn test_regex_filter() {
        test_template("{{ 'hello world' | regex('^hello') }}", "true");
        test_template("{{ 'hello world' | regex('^hello.*') }}", "true");
        test_template("{{ 'goodbye world' | regex('^hello.*') }}", "false");
        test_template("{{ 'Hello World' | regex('hello') }}", "false");
        test_template("{{ 'Hello World' | regex('(?i)hello') }}", "true");
        test_template(r"{{ 'a+b=c' | regex('\\+') }}", "true");
        test_template(r"{{ 'a+b=c' | regex('\\=') }}", "true");
        test_template("{{ 'a+b=c' | regex('d') }}", "false");
        test_template("{{ '' | regex('.*') }}", "true");
        test_template("{{ '' | regex('.+') }}", "false");
        test_template("{{ 'hello' | regex('[') }}", "false");
    }

    #[test]
    fn test_loop_controls() {
        test_template(
            "{% for i in range(1, 6) %}{% if i == 4 %}{% break %}{% endif %}{{ i }}{% if not loop.last %} {% endif %}{% endfor %}",
            "1 2 3 ",
        );
    }

    fn print_dir_diff(dir1: &Path, dir2: &Path) {
        let mut files1 = std::collections::HashSet::new();
        let mut files2 = std::collections::HashSet::new();

        for entry in WalkDir::new(dir1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let rel = entry.path().strip_prefix(dir1).unwrap().to_path_buf();
            files1.insert(rel);
        }
        for entry in WalkDir::new(dir2)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let rel = entry.path().strip_prefix(dir2).unwrap().to_path_buf();
            files2.insert(rel);
        }

        for file in files1.difference(&files2) {
            debug!("Only in {:?}: {:?}", dir1, file);
        }
        for file in files2.difference(&files1) {
            debug!("Only in {:?}: {:?}", dir2, file);
        }
        for file in files1.intersection(&files2) {
            let path1 = dir1.join(file);
            let path2 = dir2.join(file);
            let content1 = fs::read(&path1).unwrap();
            let content2 = fs::read(&path2).unwrap();
            if content1 != content2 {
                debug!("File differs: {:?}", file);
                debug!("Content in {:?}:\n{:?}", dir1, String::from_utf8(content1));
                debug!("Content in {:?}:\n{:?}", dir2, String::from_utf8(content2));
            }
        }
    }

    fn run_and_assert(template: &str, expected_dir: &str, answers: Option<&str>) {
        let tmp_dir = tempfile::tempdir().unwrap();
        let args = Args {
            template: template.to_string(),
            output_dir: tmp_dir.path().to_path_buf(),
            force: true,
            verbose: true,
            answers: answers.map(|a| a.to_string()),
            skip_confirms: vec![All],
            non_interactive: true,
        };
        run(args).unwrap();
        print_dir_diff(tmp_dir.path(), expected_dir.as_ref());
        assert!(!dir_diff::is_different(tmp_dir.path(), expected_dir).unwrap());
    }

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
