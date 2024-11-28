use baker::ignore::{ignore_file_read, IGNORE_FILE};
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_ignore_file_read() {
    let temp_dir = TempDir::new().unwrap();
    let ignore_path = temp_dir.path().join(IGNORE_FILE);

    // Test without .bakerignore
    let glob_set = ignore_file_read(&ignore_path).unwrap();
    assert!(glob_set.is_match("**/.DS_Store")); // Default pattern

    // Test with .bakerignore
    let mut file = File::create(&ignore_path).unwrap();
    writeln!(file, "*.pyc\n__pycache__/").unwrap();

    let glob_set = ignore_file_read(&ignore_path).unwrap();
    assert!(glob_set.is_match("file.pyc"));
    assert!(glob_set.is_match("__pycache__/"));
    assert!(glob_set.is_match("**/.DS_Store")); // Default pattern still works
}
