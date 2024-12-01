use std::path::PathBuf;

use baker::processor::{
    ensure_output_dir, is_jinja_template, is_rendered_path_valid, resolve_target_path,
};
use tempfile::TempDir;

#[test]
fn test_ensure_output_dir() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    // Test non-existent directory
    let new_dir = path.join("new_dir");
    assert!(ensure_output_dir(&new_dir, false).is_ok());

    // Test existing directory without force
    assert!(ensure_output_dir(path, false).is_err());

    // Test existing directory with force
    assert!(ensure_output_dir(path, true).is_ok());
}

#[test]
fn test_is_jinja_template() {
    assert!(is_jinja_template("template.html.j2"));
    assert!(is_jinja_template("file.txt.j2"));
    assert!(!is_jinja_template("regular.html"));
    assert!(!is_jinja_template("file.j2txt"));
}

#[test]
fn test_resolve_target_path() {
    let (path, should_process) = resolve_target_path("template.html.j2", "output");
    assert_eq!(path, PathBuf::from("output/template.html"));
    assert!(should_process);

    let (path, should_process) = resolve_target_path("regular.txt", "output");
    assert_eq!(path, PathBuf::from("output/regular.txt"));
    assert!(!should_process);
}

#[test]
fn test_is_rendered_path_valid() {
    assert!(!is_rendered_path_valid(""));
    assert!(!is_rendered_path_valid("output//filename.txt"));
    assert!(!is_rendered_path_valid("/filename.txt"));
    assert!(is_rendered_path_valid("filename.txt"));
    assert!(is_rendered_path_valid("output/filename.txt"));
}
