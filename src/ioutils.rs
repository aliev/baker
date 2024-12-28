use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

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

pub fn create_dir_all<P: AsRef<Path>>(dest_path: P) -> Result<()> {
    let dest_path = dest_path.as_ref();
    std::fs::create_dir_all(dest_path).map_err(Error::IoError)
}

pub fn write_file<P: AsRef<Path>>(content: &str, dest_path: P) -> Result<()> {
    let dest_path = dest_path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_path = if dest_path.is_absolute() {
        dest_path.to_path_buf()
    } else {
        base_path.join(dest_path)
    };

    if let Some(parent) = abs_path.parent() {
        create_dir_all(parent)?;
    }
    std::fs::write(abs_path, content).map_err(Error::IoError)
}

pub fn copy_file<P: AsRef<Path>>(source_path: P, dest_path: P) -> Result<()> {
    let dest_path = dest_path.as_ref();
    let source_path = source_path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_dest = if dest_path.is_absolute() {
        dest_path.to_path_buf()
    } else {
        base_path.join(dest_path)
    };

    if let Some(parent) = abs_dest.parent() {
        create_dir_all(parent)?;
    }
    std::fs::copy(source_path, abs_dest).map(|_| ()).map_err(Error::IoError)
}

pub fn read_from(
    mut reader: impl std::io::Read,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf).map_err(Error::IoError)?;

    let value = serde_json::from_str(&buf)
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    match value {
        serde_json::Value::Object(map) => Ok(map),
        _ => Ok(serde_json::Map::new()),
    }
}
