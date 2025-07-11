use baker::cli::SkipConfirm::All;
use baker::cli::{run, Args};
use log::debug;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Prints a diff of files and their contents between two directories using debug logging.
/// Shows files only present in one directory and content differences for files present in both.
///
/// # Arguments
/// * `dir1` - The first directory to compare.
/// * `dir2` - The second directory to compare.
pub fn print_dir_diff(dir1: &Path, dir2: &Path) {
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
        debug!("Only in {dir1:?}: {file:?}");
    }
    for file in files2.difference(&files1) {
        debug!("Only in {dir2:?}: {file:?}");
    }
    for file in files1.intersection(&files2) {
        let path1 = dir1.join(file);
        let path2 = dir2.join(file);
        let content1 = fs::read(&path1).unwrap();
        let content2 = fs::read(&path2).unwrap();
        if content1 != content2 {
            debug!("File differs: {file:?}");
            debug!("Content in {:?}:\n{:?}", dir1, String::from_utf8(content1));
            debug!("Content in {:?}:\n{:?}", dir2, String::from_utf8(content2));
        }
    }
}

/// Runs the baker CLI with the given template and answers, compares the output to the expected directory,
/// prints any differences, and asserts that the directories are identical.
///
/// # Arguments
/// * `template` - Path to the template directory.
/// * `expected_dir` - Path to the directory with expected output.
/// * `answers` - Optional answers for non-interactive prompts.
pub fn run_and_assert(template: &str, expected_dir: &str, answers: Option<&str>) {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: template.to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: 2,
        answers: answers.map(|a| a.to_string()),
        skip_confirms: vec![All],
        non_interactive: true,
        dry_run: false,
    };
    run(args).unwrap();
    let result = dir_diff::is_different(tmp_dir.path(), expected_dir);
    match result {
        Ok(different) => {
            if different {
                print_dir_diff(tmp_dir.path(), expected_dir.as_ref());
            }
        }
        Err(e) => {
            debug!("Error comparing directories: {e}");
        }
    }
    assert!(!dir_diff::is_different(tmp_dir.path(), expected_dir).unwrap());
}
