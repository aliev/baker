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
/// assert!(is_jinja_template("template.html.j2"));
/// assert!(!is_jinja_template("regular.html"));
/// ```
fn is_jinja_template(filename: &str) -> bool {
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
/// let (path, should_process) = resolve_target_path("templates/index.html.j2", "output");
/// assert_eq!(path, PathBuf::from("output/templates/index.html"));
/// assert!(should_process);
/// ```
fn resolve_target_path<P: AsRef<Path>>(source_path: &str, target_dir: P) -> (PathBuf, bool) {
    // Whether the file should be processed by the template renderer.
    let mut should_be_processed = false;
    let target_dir = target_dir.as_ref();

    let target_path =
        if let Some(filename) = Path::new(source_path).file_name().and_then(|n| n.to_str()) {
            if is_jinja_template(filename) {
                // Has double extension, remove .j2
                let new_name = filename.strip_suffix(".j2").unwrap();
                should_be_processed = true;
                target_dir.join(Path::new(source_path).with_file_name(new_name))
            } else {
                target_dir.join(source_path)
            }
        } else {
            target_dir.join(source_path)
        };

    if should_be_processed {
        debug!("Writing file: {}", target_path.display());
    } else {
        debug!("Copying file: {}", target_path.display());
    }

    (target_path, should_be_processed)
}

/// Processes a template directory to generate output.
///
/// # Arguments
/// * `template_dir` - Source template directory
/// * `output_dir` - Target output directory
/// * `context` - Template variables
/// * `engine` - Template rendering engine
/// * `ignored_set` - Set of patterns to ignore
/// * `force_output_dir` - Whether to overwrite existing output
///
/// # Returns
/// * `BakerResult<PathBuf>` - Path to generated output directory
pub fn process_template<P: AsRef<Path>>(
    template_dir: P,
    output_dir: P,
    context: &serde_json::Value,
    template_renderer: &Box<dyn TemplateEngine>,
    ignored_set: GlobSet,
    force_output_dir: bool,
) -> BakerResult<PathBuf> {
    debug!("Processing template...");
    let output_dir = ensure_output_dir(output_dir, force_output_dir)?;
    let template_dir = template_dir.as_ref();

    for entry in WalkDir::new(template_dir) {
        let entry = entry.map_err(|e| BakerError::IoError(e.into()))?;
        let path = entry.path();
        let relative_path = path
            .strip_prefix(template_dir)
            .map_err(|e| BakerError::ConfigError(e.to_string()))?;
        let relative_path = relative_path
            .to_str()
            .ok_or_else(|| BakerError::ConfigError("Invalid path".to_string()))?;

        debug!("Processing source file: {}", relative_path);

        // Rendered by template renderer filename.
        let rendered_path = template_renderer.render(relative_path, context)?;

        debug!("Processed target file: {}", rendered_path);

        if ignored_set.is_match(&relative_path) {
            debug!("Skipping file {} from .bakerignore", relative_path);
            continue;
        }

        // Skip if processed path is empty (conditional template evaluated to nothing)
        if rendered_path.trim().is_empty() {
            debug!("Skipping file as processed path is empty");
            continue;
        }

        let (target_path, is_template_path) = resolve_target_path(&rendered_path, &output_dir);

        if path.is_dir() {
            create_dir_all(&target_path)?;
        } else {
            if is_template_path {
                let content = read_file(path)?;
                let final_content = template_renderer.render(&content, context)?;
                write_file(&target_path, &final_content)?;
            } else {
                // Simply copy the file without processing
                copy_file(path, &target_path)?;
            }
        }
    }
    Ok(output_dir)
}
