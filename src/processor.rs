//! Core template processing module for Baker.
//! Handles file system operations, template rendering, and output generation
//! with support for path manipulation and error handling.
use globset::GlobSet;
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::error::{BakerError, BakerResult};
use crate::template::TemplateEngine;

/// Ensures the output directory exists and is safe to write to.
///
/// # Arguments
/// * `output_dir` - Target directory path for generated output
/// * `force` - Whether to overwrite existing directory
///
/// # Returns
/// * `BakerResult<PathBuf>` - Validated output directory path
///
/// # Errors
/// * Returns `BakerError::ConfigError` if directory exists and force is false
pub fn ensure_output_dir<P: AsRef<Path>>(output_dir: P, force: bool) -> BakerResult<PathBuf> {
    let output_dir = output_dir.as_ref();
    if output_dir.exists() && !force {
        return Err(BakerError::ConfigError(format!(
            "output directory already exists: {}. Use --force to overwrite",
            output_dir.display()
        )));
    }
    Ok(output_dir.to_path_buf())
}

/// Reads a file's contents into a string.
///
/// # Arguments
/// * `path` - Path to the file to read
///
/// # Returns
/// * `BakerResult<String>` - File contents
///
/// # Errors
/// * Returns `BakerError::IoError` if file cannot be read
fn read_file<P: AsRef<Path>>(path: P) -> BakerResult<String> {
    fs::read_to_string(path).map_err(BakerError::IoError)
}

/// Writes content to a file, creating parent directories if needed.
///
/// # Arguments
/// * `path` - Target file path
/// * `content` - String content to write
///
/// # Returns
/// * `BakerResult<()>` - Success or error status
///
/// # Notes
/// - Converts relative paths to absolute using current working directory
/// - Creates parent directories automatically
fn write_file<P: AsRef<Path>>(path: P, content: &str) -> BakerResult<()> {
    let path = path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_path.join(path)
    };

    if let Some(parent) = abs_path.parent() {
        fs::create_dir_all(parent).map_err(BakerError::IoError)?;
    }
    fs::write(abs_path, content).map_err(BakerError::IoError)
}

/// Creates a directory and all its parent directories.
///
/// # Arguments
/// * `path` - Directory path to create
///
/// # Returns
/// * `BakerResult<()>` - Success or error status
fn create_dir_all<P: AsRef<Path>>(path: P) -> BakerResult<()> {
    let path = path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_path.join(path)
    };
    fs::create_dir_all(abs_path).map_err(BakerError::IoError)
}

/// Copies a file from source to destination.
///
/// # Arguments
/// * `source` - Source file path
/// * `dest` - Destination file path
///
/// # Returns
/// * `BakerResult<()>` - Success or error status
///
/// # Notes
/// - Creates parent directories automatically
fn copy_file<P: AsRef<Path>>(source: P, dest: P) -> BakerResult<()> {
    let dest = dest.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_dest = if dest.is_absolute() {
        dest.to_path_buf()
    } else {
        base_path.join(dest)
    };

    if let Some(parent) = abs_dest.parent() {
        fs::create_dir_all(parent).map_err(BakerError::IoError)?;
    }
    fs::copy(source, abs_dest)
        .map(|_| ())
        .map_err(BakerError::IoError)
}

/// Checks if a file is a Jinja2 template based on its extension.
///
/// # Arguments
/// * `filename` - Name of the file to check
///
/// # Returns
/// * `bool` - True if the file has a .j2 extension, false otherwise
///
/// # Examples
/// ```
/// use baker::processor::is_jinja_template;
/// assert!(is_jinja_template("template.html.j2"));
/// assert!(!is_jinja_template("regular.html"));
/// assert!(!is_jinja_template("regular.j2"));
/// ```
pub fn is_jinja_template(filename: &str) -> bool {
    let parts: Vec<&str> = filename.split('.').collect();
    if parts.len() > 2 && parts.last() == Some(&"j2") {
        true
    } else {
        false
    }
}

/// Resolves the target path for a template file and determines if it needs processing.
///
/// # Arguments
/// * `source_path` - Path to the source template file
/// * `target_dir` - Directory where the processed file should be placed
///
/// # Returns
/// * `(PathBuf, bool)` - Tuple containing:
///   - The resolved target path
///   - Whether the file should be processed as a template
///
/// # Notes
/// - Files with .j2 extension are processed as templates
/// - The .j2 extension is stripped from the target filename
/// - Non-template files are copied directly
///
/// # Examples
/// ```
/// use std::path::PathBuf;
/// use baker::processor::resolve_target_path;
/// let (path, should_process) = resolve_target_path("templates/index.html.j2", "output");
/// assert_eq!(path, PathBuf::from("output/templates/index.html"));
/// assert!(should_process);
/// ```
pub fn resolve_target_path<P1: AsRef<Path>, P2: AsRef<Path>>(
    source_path: P1,
    target_dir: P2,
) -> (PathBuf, bool) {
    let target_dir = target_dir.as_ref();
    let source_path = source_path.as_ref();

    // Get filename if it exists, otherwise return unprocessed path
    let filename = match source_path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return (target_dir.join(source_path), false),
    };

    // Check if file is a template
    if !is_jinja_template(filename) {
        return (target_dir.join(source_path), false);
    }

    // Process template file by removing .j2 extension
    let new_name = filename.strip_suffix(".j2").unwrap();
    let target_path = target_dir.join(source_path.with_file_name(new_name));

    debug!(
        "Template file detected: {} -> {}",
        filename,
        target_path.display()
    );
    (target_path, true)
}

/// Validates whether the rendered path is valid.
pub fn is_rendered_path_valid(rendered_path: &str) -> bool {
    // Split the path by "/" and collect non-empty segments.
    let path_parts = rendered_path.split('/');

    let empty_parts: Vec<&str> = path_parts
        .clone()
        .filter(|part| part.trim().is_empty())
        .collect();

    // If any segment is empty after trimming, return an error.
    empty_parts.is_empty()
}

/// Converts a rendered path to a path relative to the template directory.
///
/// # Arguments
/// * `rendered_path` - A string representation of the path to be converted
/// * `template_dir` - The base directory to make the path relative to
///
/// # Returns
/// * `BakerResult<String>` - The relative path as a string
///
/// Example
///
/// rendered_path: examples/python-package/tests/__init__.py
/// template_dir: examples/python-package
/// ->
/// tests/__init__.py
fn get_relative_path<P: AsRef<Path>>(rendered_path: P, template_dir: P) -> BakerResult<PathBuf> {
    // // Split the path by "/" and collect non-empty segments.
    // let path_parts = rendered_path.split('/');
    // let path_parts: Vec<&str> = path_parts.collect();

    // // Join the non-empty segments back into a path string.
    // let valid_path = path_parts.join("/");

    // Convert the valid path to a Path object and attempt to strip the template directory prefix.
    let relative_path = rendered_path
        .as_ref()
        .strip_prefix(template_dir)
        .map_err(|e| BakerError::TemplateError(e.to_string()))?;

    Ok(relative_path.to_path_buf())
}

/// Processes a single template file.
///
/// # Arguments
/// * `path` - Path to the template file
/// * `target_path` - Target output path
/// * `context` - Template context
/// * `engine` - Template rendering engine
///
/// # Returns
/// * `BakerResult<()>` - Success or error status
fn process_template_file<P: AsRef<Path>>(
    path: P,
    target_path: P,
    context: &serde_json::Value,
    engine: &Box<dyn TemplateEngine>,
) -> BakerResult<()> {
    let content = read_file(&path)?;
    let final_content = engine.render(&content, context)?;
    write_file(&target_path, &final_content)?;
    Ok(())
}

/// Process a file entry
fn process_file(
    source: &Path,
    target: &Path,
    needs_processing: bool,
    context: &serde_json::Value,
    engine: &Box<dyn TemplateEngine>,
) -> BakerResult<()> {
    if needs_processing {
        process_template_file(source, target, context, engine)?
    } else {
        copy_file(source, target)?
    }
    Ok(())
}

/// Processes a single entry in the template directory
fn process_entry(
    entry: walkdir::DirEntry,
    template_dir: &Path,
    output_dir: &Path,
    context: &serde_json::Value,
    engine: &Box<dyn TemplateEngine>,
    ignored_set: &GlobSet,
) -> BakerResult<()> {
    let path = entry.path();

    // Get path relative to template directory
    let relative_to_template = path
        .strip_prefix(template_dir)
        .map_err(|e| BakerError::TemplateError(format!("Failed to strip prefix: {}", e)))?;

    // Check if file should be ignored
    if ignored_set.is_match(relative_to_template) {
        return Err(BakerError::TemplateError(format!(
            "Skipping ignored file: {}",
            relative_to_template.display()
        )));
    }

    let path_str = path
        .to_str()
        .ok_or_else(|| BakerError::TemplateError(format!("Invalid path: {}", path.display())))?;

    let rendered_path = engine.render(path_str, context).map_err(|e| {
        BakerError::TemplateError(format!("Failed to render path {}: {}", path_str, e))
    })?;

    // Validate rendered path
    if !is_rendered_path_valid(&rendered_path) {
        return Err(BakerError::TemplateError(format!(
            "Invalid rendered path: {}",
            rendered_path
        )));
    }

    // Convert rendered string back to Path
    let rendered_path = Path::new(&rendered_path);

    let relative_path = get_relative_path(rendered_path, template_dir)?;

    // Resolve final target path
    let (target_path, needs_processing) = resolve_target_path(&relative_path, output_dir);

    // Process directory or file
    if path.is_dir() {
        create_dir_all(&target_path)?
    } else {
        process_file(path, &target_path, needs_processing, context, engine)?
    }

    Ok(())
}

pub fn process_template<P: AsRef<Path>>(
    template_dir: P,
    output_dir: P,
    context: &serde_json::Value,
    engine: &Box<dyn TemplateEngine>,
    ignored_set: GlobSet,
) {
    debug!("Processing template...");
    let template_dir = template_dir.as_ref();
    let output_dir = output_dir.as_ref();

    for entry in WalkDir::new(template_dir) {
        match entry {
            Ok(entry) => {
                if let Err(e) = process_entry(
                    entry,
                    template_dir,
                    output_dir,
                    context,
                    engine,
                    &ignored_set,
                ) {
                    match e {
                        BakerError::TemplateError(msg) => {
                            log::warn!("Template processing: {}", msg)
                        }
                        BakerError::IoError(e) => log::error!("IO error: {}", e),
                        _ => log::error!("Unexpected error: {}", e),
                    }
                }
            }
            Err(e) => log::error!("Failed to access entry: {}", e),
        }
    }
}
