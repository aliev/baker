//! Core template processing module for Baker.
//! Handles file system operations, template rendering, and output generation
//! with support for path manipulation and error handling.
use dialoguer::Confirm;
use globset::GlobSet;
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::renderer::TemplateRenderer;

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
pub fn get_output_dir<P: AsRef<Path>>(output_dir: P, force: bool) -> Result<PathBuf> {
    let output_dir = output_dir.as_ref();
    if output_dir.exists() && !force {
        return Err(Error::OutputDirectoryExistsError {
            output_dir: output_dir.display().to_string(),
        });
    }
    Ok(output_dir.to_path_buf())
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
fn write_file<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    let path = path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_path =
        if path.is_absolute() { path.to_path_buf() } else { base_path.join(path) };

    if let Some(parent) = abs_path.parent() {
        fs::create_dir_all(parent).map_err(Error::IoError)?;
    }
    fs::write(abs_path, content).map_err(Error::IoError)
}

/// Creates a directory and all its parent directories.
///
/// # Arguments
/// * `path` - Directory path to create
///
/// # Returns
/// * `BakerResult<()>` - Success or error status
fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_path =
        if path.is_absolute() { path.to_path_buf() } else { base_path.join(path) };
    fs::create_dir_all(abs_path).map_err(Error::IoError)
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
fn copy_file<P: AsRef<Path>>(source: P, dest: P) -> Result<()> {
    let dest = dest.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_dest =
        if dest.is_absolute() { dest.to_path_buf() } else { base_path.join(dest) };

    if let Some(parent) = abs_dest.parent() {
        fs::create_dir_all(parent).map_err(Error::IoError)?;
    }
    fs::copy(source, abs_dest).map(|_| ()).map_err(Error::IoError)
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
/// use baker::processor::is_template_file;
/// assert!(is_template_file("template.html.j2"));
/// assert!(!is_template_file("regular.html"));
/// assert!(!is_template_file("regular.j2"));
/// ```
pub fn is_template_file(filename: &str) -> bool {
    let parts: Vec<&str> = filename.split('.').collect();
    parts.len() > 2 && parts.last() == Some(&"j2")
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
pub fn resolve_target_path<P: AsRef<Path>>(
    source_path: P,
    target_dir: P,
) -> (PathBuf, bool) {
    let target_dir = target_dir.as_ref();
    let source_path = source_path.as_ref();

    // Get filename if it exists, otherwise return unprocessed path
    let filename = match source_path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return (target_dir.join(source_path), false),
    };

    // Check if file is a template
    if !is_template_file(filename) {
        return (target_dir.join(source_path), false);
    }

    // Process template file by removing .j2 extension
    let new_name = filename.strip_suffix(".j2").unwrap();
    let target_path = target_dir.join(source_path.with_file_name(new_name));

    debug!("Template file detected: {} -> {}", filename, target_path.display());
    (target_path, true)
}

/// Validates whether the rendered path is valid.
pub fn is_rendered_path_valid<S: Into<String>>(rendered_path: S) -> bool {
    let rendered_path = rendered_path.into();
    // Split the path by "/" and collect non-empty segments.
    let path_parts = rendered_path.split('/');

    let empty_parts: Vec<&str> =
        path_parts.clone().filter(|part| part.trim().is_empty()).collect();

    // If any segment is empty after trimming, return an error.
    empty_parts.is_empty()
}

/// Process a file entry
fn process_file<P: AsRef<Path>>(
    source: P,
    target: P,
    answers: &serde_json::Value,
    engine: &dyn TemplateRenderer,
    overwrite: Option<bool>,
) -> Result<()> {
    let source = source.as_ref();
    let target = target.as_ref();
    if target.exists() {
        let confirm = match overwrite {
            Some(val) => val,
            _ => Confirm::new()
                .with_prompt(format!("Overwrite '{}'?", target.display()))
                .default(false)
                .interact()
                .map_err(Error::PromptError)?,
        };

        if !confirm {
            println!("Skipping: '{}'.", target.display());
            return Ok(());
        }
    }

    if target.exists() {
        println!("Overwriting: '{}'.", target.display());
    } else {
        println!("Creating: '{}'.", target.display());
    }
    let content = fs::read_to_string(source).map_err(Error::IoError)?;
    let rendered_content = engine.render(&content, answers)?;
    write_file(target, &rendered_content)?;
    Ok(())
}

/// Processes a single entry in the template directory
pub fn process_directory<P: AsRef<Path>>(
    source_path: P,
    template_dir: P,
    output_dir: P,
    answers: &serde_json::Value,
    engine: &dyn TemplateRenderer,
    ignored_set: &GlobSet,
    overwrite: Option<bool>,
) -> Result<()> {
    let template_dir = template_dir.as_ref();
    let output_dir = output_dir.as_ref();
    let source_path = source_path.as_ref();

    // Get path relative to template directory
    let relative_to_template =
        source_path.strip_prefix(template_dir).map_err(|e| Error::ProcessError {
            source_path: source_path.display().to_string(),
            e: e.to_string(),
        })?;

    // Check if file should be ignored
    // Skip when ignored
    if ignored_set.is_match(relative_to_template) {
        println!("Skipping: '{}'.", relative_to_template.display());
        return Ok(());
    }

    // Skip when the path is not valid
    let path_str = source_path.to_str().ok_or_else(|| Error::ProcessError {
        source_path: source_path.display().to_string(),
        e: "Cannot convert source_path to string.".to_string(),
    })?;

    // Skip when it cannot render
    let rendered_path_str = engine.render(path_str, answers).map_err(|e| {
        Error::ProcessError { source_path: path_str.to_string(), e: e.to_string() }
    })?;

    // Validate rendered path
    if !is_rendered_path_valid(&rendered_path_str) {
        return Err(Error::ProcessError {
            source_path: rendered_path_str,
            e: "The rendered path is not valid".to_string(),
        });
    }

    // Convert rendered string back to Path and get relative path
    let rendered_path = PathBuf::from(&rendered_path_str);

    // Removes template_dir prefix from rendered_path
    let relative_path =
        rendered_path.strip_prefix(template_dir).map_err(|e| Error::ProcessError {
            source_path: source_path.display().to_string(),
            e: e.to_string(),
        })?;

    // Resolve final target path
    let (target_path, needs_processing) = resolve_target_path(relative_path, output_dir);

    // Process directory or file
    if target_path.is_dir() {
        create_dir_all(&target_path)?;
        return Ok(());
    }

    if needs_processing {
        process_file(source_path, &target_path, answers, engine, overwrite)?
    } else {
        copy_file(source_path, &target_path)?
    }

    Ok(())
}
