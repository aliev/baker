//! Core template processing module for Baker.
//! Handles file system operations, template rendering, and output generation
//! with support for path manipulation and error handling.
use globset::GlobSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::prompt::prompt_confirm;
use crate::renderer::TemplateRenderer;

#[derive(Debug)]
enum FileAction {
    Create,
    Overwrite,
    Skip,
}

impl std::fmt::Display for FileAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileAction::Create => write!(f, "Creating"),
            FileAction::Overwrite => write!(f, "Overwriting"),
            FileAction::Skip => write!(f, "Skipping"),
        }
    }
}

fn determine_file_action(
    target_path_exists: bool,
    user_confirmed_overwrite: bool,
) -> FileAction {
    match (target_path_exists, user_confirmed_overwrite) {
        (true, true) => FileAction::Overwrite,
        (false, _) => FileAction::Create,
        (true, false) => FileAction::Skip,
    }
}

fn has_valid_rendered_path_parts<S: Into<String>>(
    template_path: S,
    rendered_path: S,
) -> bool {
    let template_path = template_path.into();
    let rendered_path = rendered_path.into();
    let template_path: Vec<&str> = template_path.split('/').collect();
    let rendered_path: Vec<&str> = rendered_path.split('/').collect();

    for (template_path, rendered_path) in template_path.iter().zip(rendered_path.iter()) {
        if is_template_string(template_path) && rendered_path.is_empty() {
            return false;
        }
    }

    true
}

fn write_file<P: AsRef<Path>>(target_path: P, content: &str) -> Result<()> {
    let target_path = target_path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_path = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        base_path.join(target_path)
    };

    if let Some(parent) = abs_path.parent() {
        fs::create_dir_all(parent).map_err(Error::IoError)?;
    }
    fs::write(abs_path, content).map_err(Error::IoError)
}

fn copy_file<P: AsRef<Path>>(source_path: P, dest_path: P) -> Result<()> {
    let dest_path = dest_path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_dest = if dest_path.is_absolute() {
        dest_path.to_path_buf()
    } else {
        base_path.join(dest_path)
    };

    if let Some(parent) = abs_dest.parent() {
        fs::create_dir_all(parent).map_err(Error::IoError)?;
    }
    fs::copy(source_path, abs_dest).map(|_| ()).map_err(Error::IoError)
}

fn is_template_file<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    let parts: Vec<&str> = file_name.split('.').collect();
    parts.len() > 2 && parts.last() == Some(&"j2")
}

fn is_template_string(text: &str) -> bool {
    text.starts_with("{%") && text.ends_with("%}")
        || text.starts_with("{{") && text.ends_with("}}")
}

fn render_path<P: AsRef<Path>>(
    template_path: P,
    answers: &serde_json::Value,
    engine: &dyn TemplateRenderer,
) -> Result<String> {
    // Convert the input to a Path
    let template_path = template_path.as_ref();

    // Ensure the path can be converted to a string
    let path_str = template_path.to_str().ok_or_else(|| Error::ProcessError {
        source_path: template_path.display().to_string(),
        e: "Cannot convert source_path to string.".to_string(),
    })?;

    // Render the path using the template engine
    engine.render(path_str, answers).map_err(|e| Error::ProcessError {
        source_path: path_str.to_string(),
        e: e.to_string(),
    })
}

/// Renders the content of a template file using provided answers.
///
/// # Arguments
/// * `template_path` - Path to the template file
/// * `answers` - JSON data for template variable substitution
/// * `engine` - Template rendering engine implementation
///
/// # Returns
/// * `Ok(String)` containing the rendered content
/// * `Err(Error)` if file reading or template processing fails
fn render_content<P: AsRef<Path>>(
    template_path: P,
    answers: &serde_json::Value,
    engine: &dyn TemplateRenderer,
) -> Result<String> {
    let template_path = template_path.as_ref();
    let content = fs::read_to_string(template_path).map_err(Error::IoError)?;
    engine.render(&content, answers)
}

/// Processes a single template directory entry, handling both files and directories.
///
/// # Arguments
/// * `template_path` - Path to the template entry to process
/// * `template_root` - Root directory containing templates
/// * `output_root` - Target directory for processed output
/// * `answers` - JSON data for template variable substitution
/// * `engine` - Template rendering engine implementation
/// * `ignored_patterns` - Set of glob patterns for files to ignore
/// * `skip_overwrite_check` - Whether to overwrite existing files without prompting
///
/// # Returns
/// * `Ok(())` if processing succeeds
/// * `Err(Error)` if any step of processing fails
pub fn process_template_entry<P: AsRef<Path>>(
    template_root: P,
    output_root: P,
    template_entry: P,
    answers: &serde_json::Value,
    engine: &dyn TemplateRenderer,
    ignored_patterns: &GlobSet,
    skip_overwrite_check: bool,
) -> Result<()> {
    // Relative or absolute path to the template root.
    let template_root = template_root.as_ref();
    // Relative or absolute path to the output root.
    let output_root = output_root.as_ref();
    // Relative or absolute path to every file or directory
    // in the template root.
    let template_entry = template_entry.as_ref();

    // Skip processing if template entry in `.bakerignore`
    if ignored_patterns.is_match(template_entry) {
        println!("Skipping (.bakerignore): '{}'.", template_entry.display());
        return Ok(());
    }

    // Build the rendered path from template_path
    let rendered_entry = render_path(template_entry, answers, engine)?;
    let rendered_entry = rendered_entry.as_str();

    // Skip when the path is not valid
    if !has_valid_rendered_path_parts(
        rendered_entry,
        template_entry.to_str().unwrap_or_default(),
    ) {
        return Err(Error::ProcessError {
            source_path: rendered_entry.to_string(),
            e: "The rendered path is not valid".to_string(),
        });
    }

    // In order to build target path we have to remove the template suffix if it exists.
    // If template suffix does not exist we keep the path as it is.
    let rendered_path = if is_template_file(rendered_entry) {
        rendered_entry.strip_suffix(".j2").unwrap_or_default()
    } else {
        rendered_entry
    };

    let rendered_path_buf = PathBuf::from(rendered_path);

    // Creating the target path
    let target_path = rendered_path_buf.strip_prefix(template_root).map_err(|e| {
        Error::ProcessError {
            source_path: template_entry.display().to_string(),
            e: e.to_string(),
        }
    })?;

    let target_path = output_root.join(target_path);

    let rendered_content = render_content(template_entry, answers, engine)?;

    let prompt_not_needed = skip_overwrite_check || !target_path.exists();

    let user_confirmed_overwrite = prompt_confirm(
        prompt_not_needed,
        format!("Overwrite {}?", target_path.display()),
    )?;

    let action = determine_file_action(target_path.exists(), user_confirmed_overwrite);

    match action {
        FileAction::Create | FileAction::Overwrite => {
            if is_template_file(template_entry) {
                // Just copies file as it is.
                copy_file(template_entry, &target_path)?;
            } else {
                // Copies file with writing a rendered content.
                write_file(target_path, &rendered_content)?;
            }
            Ok(())
        }
        FileAction::Skip => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use crate::renderer::MiniJinjaRenderer;
    use serde_json::json;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_render_path() {
        let engine = Box::new(MiniJinjaRenderer::new());
        let template_path = "template/{{variable}}.txt";
        let answers = json!({"variable": "Hello, World"});
        let rendered_path = render_path(template_path, &answers, &*engine);
        assert_eq!(rendered_path.unwrap(), "template/Hello, World.txt".to_string());
    }

    #[test]
    fn test_render_path_empty_variable() {
        let engine = Box::new(MiniJinjaRenderer::new());
        let template_path = "template/{{variable}}.txt";
        let answers = json!({});
        let rendered_path = render_path(template_path, &answers, &*engine);
        assert_eq!(rendered_path.unwrap(), "template/.txt".to_string());
    }

    #[test]
    fn test_render_content() {
        let engine = Box::new(MiniJinjaRenderer::new());
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("template.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, {{placeholder}}!").unwrap();
        let answers = json!({"placeholder": "World"});
        let result = render_content(&file_path, &answers, &*engine);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[test]
    fn test_is_template_file() {
        assert!(is_template_file("template.html.j2"));
        assert!(!is_template_file("regular.html"));
        assert!(!is_template_file("regular.j2"));
    }

    #[test]
    fn test_process_template_entry() {
        let engine = Box::new(MiniJinjaRenderer::new());
        let dir = tempdir().unwrap();
        let template_dir = dir.path().join("template");
        let output_dir = dir.path().join("output");

        // Create template file
        fs::create_dir_all(&template_dir).unwrap();
        let template_file = template_dir.join("hello.txt.j2");
        let mut file = File::create(&template_file).unwrap();
        file.write_all(b"Hello, {{name}}!").unwrap();

        // Process template
        let answers = json!({"name": "World"});
        let ignored_set = GlobSet::empty();
        let result = process_template_entry(
            &template_file,
            &template_dir,
            &output_dir,
            &answers,
            &*engine,
            &ignored_set,
            true,
        );

        assert!(result.is_ok());

        // Verify output
        let output_file = output_dir.join("hello.txt");
        assert!(output_file.exists());
        let content = fs::read_to_string(output_file).unwrap();
        assert_eq!(content, "Hello, World!");
    }
}
