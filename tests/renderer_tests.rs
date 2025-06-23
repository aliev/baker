#[cfg(test)]
mod tests {
    use baker::cli::{run, Args, SkipConfirm::All};
    use baker::renderer::{MiniJinjaRenderer, TemplateRenderer};
    use serde_json::json;
    use test_log::test;

    fn test_template(template: &str, expected: &str) {
        let renderer = MiniJinjaRenderer::new();
        let result = renderer.render(template, &json!({}), None).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_camel_case_filter() {
        test_template("{{ 'hello world' | camel_case }}", "helloWorld");
    }

    #[test]
    fn test_kebab_case_filter() {
        test_template("{{ 'hello world' | kebab_case }}", "hello-world");
    }

    #[test]
    fn test_pascal_case_filter() {
        test_template("{{ 'hello world' | pascal_case }}", "HelloWorld");
    }

    #[test]
    fn test_screaming_snake_case_filter() {
        test_template("{{ 'hello world' | screaming_snake_case }}", "HELLO_WORLD");
    }

    #[test]
    fn test_snake_case_filter() {
        test_template("{{ 'hello world' | snake_case }}", "hello_world");
    }

    #[test]
    fn test_table_case_filter() {
        test_template("{{ 'Hello World' | table_case }}", "hello_worlds");
    }

    #[test]
    fn test_train_case_filter() {
        test_template("{{ 'hello world' | train_case }}", "Hello-World");
    }

    #[test]
    fn test_plural_filter() {
        test_template("{{ 'car' | plural }}", "cars");
    }

    #[test]
    fn test_singular_filter() {
        test_template("{{ 'cars' | singular }}", "car");
    }

    #[test]
    fn test_foreign_key_filter() {
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
    }

    #[test]
    fn test_loop_controls() {
        test_template(
            "{% for i in range(1, 6) %}{% if i == 4 %}{% break %}{% endif %}{{ i }}{% if not loop.last %} {% endif %}{% endfor %}",
            "1 2 3 ",
        );
    }

    #[test]
    fn test_regex_filter_invalid_regex() {
        let renderer = MiniJinjaRenderer::new();
        let result = renderer.render("{{ 'hello' | regex('[') }}", &json!({}), None);
        assert_eq!(result.unwrap(), "false");
    }

    #[test]
    fn test_demo_copy() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let args = Args {
            template: "examples/demo".to_string(),
            output_dir: tmp_dir.path().to_path_buf(),
            force: true,
            verbose: true,
            answers: Some("{\"project_name\": \"demo\", \"project_author\": \"demo\", \"project_slug\": \"demo\", \"use_tests\": true}".to_string()),
            skip_confirms: vec![All],
            non_interactive: true,
        };
        run(args).unwrap();
        assert!(!dir_diff::is_different(tmp_dir.path(), "tests/expected/demo").unwrap());
    }

    #[test]
    fn test_demo_copy_use_tests_false() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let args = Args {
            template: "examples/demo".to_string(),
            output_dir: tmp_dir.path().to_path_buf(),
            force: true,
            verbose: true,
            answers: Some("{\"project_name\": \"demo\", \"project_author\": \"demo\", \"project_slug\": \"demo\", \"use_tests\": false}".to_string()),
            skip_confirms: vec![All],
            non_interactive: true,
        };
        run(args).unwrap();
        assert!(!dir_diff::is_different(
            tmp_dir.path(),
            "tests/expected/demo_tests_false"
        )
        .unwrap());
    }

    #[test]
    fn test_filters_example() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let args = Args {
            template: "examples/filters".to_string(),
            output_dir: tmp_dir.path().to_path_buf(),
            force: true,
            verbose: true,
            answers: Some("{\"project_name\": \"project name is filters\"}".to_string()),
            skip_confirms: vec![All],
            non_interactive: true,
        };
        run(args).unwrap();
        assert!(
            !dir_diff::is_different(tmp_dir.path(), "tests/expected/filters").unwrap()
        );
    }

    #[test]
    fn test_jsonschema_default() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let args = Args {
            template: "tests/templates/jsonschema".to_string(),
            output_dir: tmp_dir.path().to_path_buf(),
            force: true,
            verbose: true,
            answers: None,
            skip_confirms: vec![All],
            non_interactive: true,
        };
        run(args).unwrap();
        assert!(!dir_diff::is_different(
            tmp_dir.path(),
            "tests/expected/jsonschema-default"
        )
        .unwrap());
    }

    #[test]
    fn test_jsonschema() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let args = Args {
            template: "tests/templates/jsonschema".to_string(),
            output_dir: tmp_dir.path().to_path_buf(),
            force: true,
            verbose: true,
            answers: Some("{\"database_config\":{\"engine\":\"redis\",\"host\":\"localhost\",\"port\":6379}}".to_string()),
            skip_confirms: vec![All],
            non_interactive: true,
        };
        run(args).unwrap();
        assert!(
            !dir_diff::is_different(tmp_dir.path(), "tests/expected/jsonschema").unwrap()
        );
    }

    #[test]
    fn test_import() {
        let _ = env_logger::try_init();
        let tmp_dir = tempfile::tempdir().unwrap();
        let args = Args {
            template: "examples/import".to_string(),
            output_dir: tmp_dir.path().to_path_buf(),
            force: true,
            verbose: true,
            answers: Some("{\"database_config\":{\"engine\":\"redis\",\"host\":\"localhost\",\"port\":6379}}".to_string()),
            skip_confirms: vec![All],
            non_interactive: true,
        };
        run(args).unwrap();
        assert!(!dir_diff::is_different(tmp_dir.path(), "tests/expected/import").unwrap());
    }

    #[test]
    fn test_import_directory() {
        let _ = env_logger::try_init();
        let tmp_dir = tempfile::tempdir().unwrap();
        let args = Args {
            template: "examples/import_directory".to_string(),
            output_dir: tmp_dir.path().to_path_buf(),
            force: true,
            verbose: true,
            answers: None,
            skip_confirms: vec![All],
            non_interactive: true,
        };
        run(args).unwrap();
        assert!(!dir_diff::is_different(
            tmp_dir.path(),
            "tests/expected/import_directory"
        )
        .unwrap());
    }
}
