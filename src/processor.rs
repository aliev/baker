use globset::GlobSet;
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::{
    error::{BakerError, BakerResult},
    render::TemplateRenderer,
};

pub fn get_output_dir<P: AsRef<Path>>(output_dir: P, force: bool) -> BakerResult<PathBuf> {
    let output_dir = output_dir.as_ref();
    if output_dir.exists() && !force {
        return Err(BakerError::ConfigError(format!(
            "output directory already exists: {}. Use --force to overwrite",
            output_dir.display()
        )));
    }
    Ok(output_dir.to_path_buf())
}

fn read_file<P: AsRef<Path>>(path: P) -> BakerResult<String> {
    fs::read_to_string(path).map_err(BakerError::IoError)
}

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

fn is_template_path(filename: &str) -> bool {
    let parts: Vec<&str> = filename.split('.').collect();
    if parts.len() > 2 && parts.last() == Some(&"j2") {
        true
    } else {
        false
    }
}

fn get_target_path<P: AsRef<Path>>(processed_path: &str, target_dir: P) -> (PathBuf, bool) {
    // Whether the file should be processed by the template renderer.
    let mut should_be_processed = false;
    let target_dir = target_dir.as_ref();

    let target_path = if let Some(filename) = Path::new(processed_path)
        .file_name()
        .and_then(|n| n.to_str())
    {
        if is_template_path(filename) {
            // Has double extension, remove .j2
            let new_name = filename.strip_suffix(".j2").unwrap();
            should_be_processed = true;
            target_dir.join(Path::new(processed_path).with_file_name(new_name))
        } else {
            target_dir.join(processed_path)
        }
    } else {
        target_dir.join(processed_path)
    };

    if should_be_processed {
        debug!("Writing file: {}", target_path.display());
    } else {
        debug!("Copying file: {}", target_path.display());
    }

    (target_path, should_be_processed)
}

pub fn process_template<P: AsRef<Path>>(
    template_dir: P,
    output_dir: P,
    context: &serde_json::Value,
    template_renderer: &Box<dyn TemplateRenderer>,
    bakerignore: GlobSet,
    force_output_dir: bool,
) -> BakerResult<PathBuf> {
    debug!("Processing template...");
    let output_dir = get_output_dir(output_dir, force_output_dir)?;
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

        if bakerignore.is_match(&relative_path) {
            debug!("Skipping file {} from .bakerignore", relative_path);
            continue;
        }

        // Skip if processed path is empty (conditional template evaluated to nothing)
        if rendered_path.trim().is_empty() {
            debug!("Skipping file as processed path is empty");
            continue;
        }

        let (target_path, is_template_path) = get_target_path(&rendered_path, &output_dir);

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
