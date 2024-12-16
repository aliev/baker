//! Core template processing module for Baker.
//! Handles file system operations, template rendering, and output generation
//! with support for path manipulation and error handling.
use globset::GlobSet;
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::prompt::prompt_confirm;
use crate::renderer::TemplateRenderer;

fn has_valid_rendered_path_parts(template_path: &str, rendered_path: &str) -> bool {
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

fn create_dir_all<P: AsRef<Path>>(dir_path: P) -> Result<()> {
    let dir_path = dir_path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_path = if dir_path.is_absolute() {
        dir_path.to_path_buf()
    } else {
        base_path.join(dir_path)
    };
    fs::create_dir_all(abs_path).map_err(Error::IoError)
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

pub fn resolve_target_path<P: AsRef<Path>>(
    source_path: P,
    output_dir: P,
) -> (PathBuf, bool) {
    let source_path = source_path.as_ref();
    let output_dir = output_dir.as_ref();

    // Get filename if it exists, otherwise return unprocessed path
    let filename = match source_path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return (output_dir.join(source_path), false),
    };

    // Check if file is a template
    if !is_template_file(filename) {
        return (output_dir.join(source_path), false);
    }

    // Process template file by removing .j2 extension
    let new_name = filename.strip_suffix(".j2").unwrap();
    let target_path = output_dir.join(source_path.with_file_name(new_name));

    debug!("Template file detected: {} -> {}", filename, target_path.display());
    (target_path, true)
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
    Ok(engine.render(&content, answers)?)
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
/// * `force_overwrite` - Whether to overwrite existing files without prompting
///
/// # Returns
/// * `Ok(())` if processing succeeds
/// * `Err(Error)` if any step of processing fails
pub fn process_template_entry<P: AsRef<Path>>(
    template_path: P,
    template_root: P,
    output_root: P,
    answers: &serde_json::Value,
    engine: &dyn TemplateRenderer,
    ignored_patterns: &GlobSet,
    force_overwrite: bool,
) -> Result<()> {
    let template_root = template_root.as_ref();
    let output_root = output_root.as_ref();
    let template_path = template_path.as_ref();

    let rendered_path = render_path(template_path, answers, engine)?;

    let p1 = rendered_path.as_str();
    let p2 = template_path.to_str().unwrap();

    // Skip when the path is not valid
    if !has_valid_rendered_path_parts(p1, p2) {
        return Err(Error::ProcessError {
            source_path: rendered_path,
            e: "The rendered path is not valid".to_string(),
        });
    }

    let rendered_path = PathBuf::from(rendered_path);

    // Here we starting the process of building the entry path in output directory.
    // We should remove the template directory prefix from the `template_path`.
    // For example, if your `template_path` is: `my_template_directory/README.md`
    // the path will be converted to `output_root/README.md`.
    let target_entry =
        rendered_path.strip_prefix(template_root).map_err(|e| Error::ProcessError {
            source_path: template_path.display().to_string(),
            e: e.to_string(),
        })?;

    // Skip processing if template entry in `.bakerignore`
    if ignored_patterns.is_match(target_entry) {
        println!("Skipping (.bakerignore): '{}'.", target_entry.display());
        return Ok(());
    }

    // Resolve final target path
    let (target_path, needs_processing) = resolve_target_path(target_entry, output_root);

    // Process directory or file
    if target_path.is_dir() {
        create_dir_all(&target_path)?;
        return Ok(());
    }

    if !needs_processing {
        copy_file(template_path, &target_path)?;
        return Ok(());
    }

    let rendered_content = render_content(template_path, answers, engine)?;

    if target_path.exists() {
        let overwrite = prompt_confirm(
            force_overwrite,
            format!("Overwrite {}?", target_path.display()),
        )?;

        if overwrite {
            println!("Overwriting: '{}'.", target_path.display());
        } else {
            println!("Skipping: '{}'.", target_path.display());
            return Ok(());
        }
    } else {
        println!("Creating: '{}'.", target_path.display());
    }
    write_file(target_path, &rendered_content)?;

    Ok(())
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
