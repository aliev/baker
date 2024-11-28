use baker::template::{
    LocalLoader, MiniJinjaEngine, TemplateEngine, TemplateLoader, TemplateSource,
};
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_template_source_from_string() {
    match TemplateSource::from_string("https://github.com/user/repo.git") {
        Some(TemplateSource::Git(url)) => assert_eq!(url, "https://github.com/user/repo.git"),
        _ => panic!("Expected Git source"),
    }

    match TemplateSource::from_string("git@github.com:user/repo.git") {
        Some(TemplateSource::Git(url)) => assert_eq!(url, "git@github.com:user/repo.git"),
        _ => panic!("Expected Git source"),
    }

    match TemplateSource::from_string("./local/path") {
        Some(TemplateSource::FileSystem(path)) => {
            assert_eq!(path, PathBuf::from("./local/path"))
        }
        _ => panic!("Expected FileSystem source"),
    }
}

#[test]
fn test_local_loader() {
    let temp_dir = TempDir::new().unwrap();
    let loader = LocalLoader::new();

    match loader.load(&TemplateSource::FileSystem(temp_dir.path().to_path_buf())) {
        Ok(path) => assert_eq!(path, temp_dir.path()),
        Err(_) => panic!("Expected successful load"),
    }
}

#[test]
fn test_minijinja_engine() {
    let engine = MiniJinjaEngine::new();
    let context = serde_json::json!({
        "name": "test",
        "value": 42
    });

    let result = engine.render("Hello {{ name }}!", &context).unwrap();
    assert_eq!(result, "Hello test!");

    let result = engine.render("Value: {{ value }}", &context).unwrap();
    assert_eq!(result, "Value: 42");
}
