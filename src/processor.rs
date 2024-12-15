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

/// Checks whether the rendered path is valid.
///
/// This function verifies if a given rendered path is valid by ensuring
/// there are no empty segments in the path when split by "/".
///
/// # Example
/// A rendered path might occasionally contain empty segments, such as:
///
/// `path/to//my_file.txt`
///
/// This can happen for various reasons, for instance, when a file template
/// includes conditional placeholders:
///
/// ```text
/// template_dir/{% if create_file %}dirname{% endif %}/filename.txt
/// ```
///
/// If `create_file` is `false`, the resulting path would be:
///
/// `template_dir//filename.txt`
///
/// This function identifies such cases and returns `false` to prevent
/// the creation of invalid paths or files.
///
/// # Parameters
/// - `rendered_path`: The rendered path to be validated, provided as a type that
///   can be converted into a `String`.
///
/// # Returns
/// - `true` if the rendered path contains no empty segments after trimming.
/// - `false` if the rendered path contains one or more empty segments.
pub fn is_rendered_path_valid<S: Into<String>>(rendered_path: S) -> bool {
    let rendered_path = rendered_path.into();
    // Split the path by "/" and collect non-empty segments.
    let path_parts = rendered_path.split('/');

    let empty_parts: Vec<&str> =
        path_parts.clone().filter(|part| part.trim().is_empty()).collect();

    // If any segment is empty after trimming, return an error.
    empty_parts.is_empty()
}

/// Renders the provided template path using the given answers and template rendering engine.
///
/// This function takes a path to a template file or directory, uses the provided `answers`
/// as the rendering context, and applies the specified template rendering engine to produce
/// a rendered `PathBuf`. The resulting path is validated to ensure it is a valid file system path.
///
/// # Type Parameters
/// - `P`: A type that can be converted to a `Path` (e.g., `Path`, `PathBuf`, or `&str`).
///
/// # Arguments
/// - `template_entry`: The path to the template file or directory to render.
/// - `answers`: A JSON object containing context variables for rendering.
/// - `engine`: An implementation of the `TemplateRenderer` trait used to render the template path.
///
/// # Returns
/// - `Ok(PathBuf)`: A valid, rendered path as a `PathBuf`.
/// - `Err(Error)`: An error occurs if:
///     - The `template_entry` path cannot be converted to a string.
///     - The rendering engine encounters an error while processing the path.
///     - The rendered path is invalid (e.g., contains illegal characters or is empty).
///
/// # Errors
/// - Returns `Error::ProcessError` if:
///     - The source path cannot be converted to a valid UTF-8 string.
///     - Rendering the template path fails.
///     - The resulting rendered path is not valid.
fn render_path<P: AsRef<Path>>(
    template_entry: P,
    answers: &serde_json::Value,
    engine: &dyn TemplateRenderer,
) -> Result<String> {
    // Convert the input to a Path
    let template_entry = template_entry.as_ref();

    // Ensure the path can be converted to a string
    let path_str = template_entry.to_str().ok_or_else(|| Error::ProcessError {
        source_path: template_entry.display().to_string(),
        e: "Cannot convert source_path to string.".to_string(),
    })?;

    // Render the path using the template engine
    engine.render(path_str, answers).map_err(|e| Error::ProcessError {
        source_path: path_str.to_string(),
        e: e.to_string(),
    })
}

/// Process a file entry
fn render_content<P: AsRef<Path>>(
    template_entry: P,
    answers: &serde_json::Value,
    engine: &dyn TemplateRenderer,
) -> Result<String> {
    let template_entry = template_entry.as_ref();
    let content = fs::read_to_string(template_entry).map_err(Error::IoError)?;
    Ok(engine.render(&content, answers)?)
}

fn confirm_file_overwriting<P: AsRef<Path>>(
    target_path: P,
    overwrite: Option<bool>,
) -> Result<bool> {
    let target_path = target_path.as_ref();
    if target_path.exists() {
        let confirm = match overwrite {
            Some(val) => val,
            _ => Confirm::new()
                .with_prompt(format!("Overwrite '{}'?", target_path.display()))
                .default(false)
                .interact()
                .map_err(Error::PromptError)?,
        };
        return Ok(confirm);
    }

    Ok(true)
}

/// Processes a single entry in the template directory
///
/// # Arguments
/// * `template_entry` - An item representing a file or directory from the template directory tree.
/// * `template_dir` - The path to template directory
/// * `output_dir` - The path to output directory
pub fn process_template_entry<P: AsRef<Path>>(
    template_entry: P,
    template_dir: P,
    output_dir: P,
    answers: &serde_json::Value,
    engine: &dyn TemplateRenderer,
    ignored_set: &GlobSet,
    overwrite: Option<bool>,
) -> Result<()> {
    let template_dir = template_dir.as_ref();
    let output_dir = output_dir.as_ref();
    let template_entry = template_entry.as_ref();

    // Skip processing if template entry in `.bakerignore`
    if ignored_set.is_match(template_entry) {
        println!("Skipping: '{}'.", template_entry.display());
        return Ok(());
    }

    let rendered_path = render_path(template_entry, answers, engine)?;

    // Skip when the path is not valid
    if !is_rendered_path_valid(&rendered_path) {
        return Err(Error::ProcessError {
            source_path: rendered_path,
            e: "The rendered path is not valid".to_string(),
        });
    }

    let rendered_path = PathBuf::from(rendered_path);

    // Here we starting the process of building the entry path in output directory.
    // We should remove the template directory prefix from the `template_entry`.
    // For example, if your `template_entry` is: `my_template_directory/README.md`
    // the path will be converted to `output_dir/README.md`.
    let target_entry =
        rendered_path.strip_prefix(template_dir).map_err(|e| Error::ProcessError {
            source_path: template_entry.display().to_string(),
            e: e.to_string(),
        })?;

    // Resolve final target path
    let (target_path, needs_processing) = resolve_target_path(target_entry, output_dir);

    // Process directory or file
    if target_path.is_dir() {
        create_dir_all(&target_path)?;
        return Ok(());
    }

    if needs_processing {
        let rendered_template_entry_content = render_content(
            template_entry,
            // &target_path,
            answers,
            engine,
            // overwrite,
        )?;

        let confirm = confirm_file_overwriting(&target_path, overwrite)?;

        if !confirm {
            println!("Skipping: '{}'.", target_path.display());
            return Ok(());
        }

        if target_path.exists() {
            println!("Overwriting: '{}'.", target_path.display());
        } else {
            println!("Creating: '{}'.", target_path.display());
        }
        write_file(target_path, &rendered_template_entry_content)?;
    } else {
        copy_file(template_entry, &target_path)?
    }

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
    fn test_is_rendered_path_valid() {
        // Invalid: Contains an empty segment between slashes ("//").
        let is_valid = is_rendered_path_valid("template//filename.txt");
        assert_eq!(is_valid, false);

        // Valid: All segments are non-empty.
        let is_valid = is_rendered_path_valid("template/my_directory/filename.txt");
        assert_eq!(is_valid, true);

        // Invalid: Ends with a trailing slash, resulting in an empty segment at the end.
        let is_valid = is_rendered_path_valid("template/my_directory/");
        assert_eq!(is_valid, false);

        // Invalid: Starts with a leading slash, resulting in an empty segment at the start.
        let is_valid = is_rendered_path_valid("/template");
        assert_eq!(is_valid, false);

        // Invalid: Starts and ends with slashes, resulting in empty segments at both ends.
        let is_valid = is_rendered_path_valid("/template/");
        assert_eq!(is_valid, false);
    }
}
