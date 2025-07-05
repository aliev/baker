//! Tests for the config module

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::config::question::{Question, QuestionRendered};
    use crate::config::types::{get_default_validation, Type};
    use crate::config::QuestionType;
    use crate::template::get_template_engine;

    #[test]
    fn it_works_1() {
        let question = Question {
            help: "Hello, {{prev_answer}}".to_string(),
            r#type: Type::Bool,
            default: serde_json::Value::Null,
            ask_if: r#"prev_answer == "TEST""#.to_string(),
            secret: None,
            multiselect: false,
            choices: vec![],
            schema: None,
            validation: get_default_validation(),
        };
        let engine = get_template_engine();

        let answers = json!({
            "prev_answer": "World"
        });

        let result = question.render("question1".as_ref(), &answers, &engine);
        let QuestionRendered { ask_if, help, default, r#type } = result;
        assert!(!ask_if);
        assert_eq!(help, "Hello, World".to_string());
        assert_eq!(default, serde_json::Value::Bool(false));
        assert_eq!(r#type, QuestionType::Boolean);
    }

    #[test]
    fn it_works_2() {
        let question = Question {
            help: "{{question}}".to_string(),
            r#type: Type::Str,
            default: json!(vec!["Python".to_string(), "Django".to_string()]),
            ask_if: "".to_string(),
            secret: None,
            multiselect: true,
            choices: vec![
                "Python".to_string(),
                "Django".to_string(),
                "FastAPI".to_string(),
                "Next.JS".to_string(),
                "TypeScript".to_string(),
            ],
            schema: None,
            validation: get_default_validation(),
        };
        let engine = get_template_engine();

        let answers = json!({
            "question": "Please select your stack"
        });

        let result = question.render("question1".as_ref(), &answers, &engine);
        let QuestionRendered { ask_if, help, default, r#type } = result;
        assert!(ask_if);
        assert_eq!(help, "Please select your stack".to_string());
        assert_eq!(default, json!(vec!["Python".to_string(), "Django".to_string()]));
        assert_eq!(r#type, QuestionType::MultipleChoice);
    }

    #[test]
    fn it_works_3() {
        let question = Question {
            help: "".to_string(),
            r#type: Type::Str,
            default: serde_json::Value::Null,
            ask_if: "answer is not defined".to_string(),
            secret: None,
            multiselect: false,
            choices: vec![],
            schema: None,
            validation: get_default_validation(),
        };
        let engine = get_template_engine();

        let answers = json!({});

        let result = question.render("question1".as_ref(), &answers, &engine);
        let QuestionRendered { ask_if, r#type, .. } = result;
        assert!(ask_if);
        assert_eq!(r#type, QuestionType::Text);
    }

    #[test]
    fn it_works_4() {
        let question = Question {
            help: "".to_string(),
            r#type: Type::Str,
            default: serde_json::Value::Null,
            ask_if: "answer is not defined".to_string(),
            secret: None,
            multiselect: false,
            choices: vec![],
            schema: None,
            validation: get_default_validation(),
        };
        let engine = get_template_engine();

        let answers = json!({"answer": "Here is an answer"});

        let result = question.render("question1".as_ref(), &answers, &engine);
        let QuestionRendered { ask_if, r#type, .. } = result;
        assert!(!ask_if);
        assert_eq!(r#type, QuestionType::Text);
    }

    #[test]
    fn it_works_5() {
        let question = Question {
            help: "".to_string(),
            r#type: Type::Str,
            default: json!("This is a default value"),
            ask_if: "question1 is not defined".to_string(),
            secret: None,
            multiselect: false,
            choices: vec![],
            schema: None,
            validation: get_default_validation(),
        };
        let engine = get_template_engine();

        let answers = json!({"question1": "This is a default value for the question1"});

        let result = question.render("question1".as_ref(), &answers, &engine);
        let QuestionRendered { ask_if, r#type, default, .. } = result;
        assert!(!ask_if);
        assert_eq!(r#type, QuestionType::Text);
        assert_eq!(default, json!("This is a default value for the question1"));
    }

    #[test]
    fn it_works_6() {
        let question = Question {
            help: "".to_string(),
            r#type: Type::Str,
            default: json!("This is a default value"),
            ask_if: "question1 is not defined".to_string(),
            secret: None,
            multiselect: false,
            choices: vec![],
            schema: None,
            validation: get_default_validation(),
        };
        let engine = get_template_engine();

        let answers = json!({});

        let result = question.render("question1".as_ref(), &answers, &engine);
        let QuestionRendered { ask_if, r#type, default, .. } = result;
        assert!(ask_if);
        assert_eq!(r#type, QuestionType::Text);
        assert_eq!(default, json!("This is a default value"));
    }
}
