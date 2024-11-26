//! File and directory ignore pattern handling for Baker templates.
//! This module processes .bakerignore files to exclude specific paths
//! from template processing, similar to .gitignore functionality.

use crate::error::{BakerError, BakerResult};
use globset::{Glob, GlobSet, GlobSetBuilder};
use log::debug;
use std::{fs::read_to_string, path::Path};

/// Reads and processes the .bakerignore file to create a set of glob patterns.
///
/// # Arguments
/// * `bakerignore_path` - Path to the .bakerignore file
///
/// # Returns
/// * `BakerResult<GlobSet>` - Set of compiled glob patterns for path matching
///
/// # Notes
/// - If the .bakerignore file doesn't exist, returns an empty GlobSet
/// - Each line in the file is treated as a separate glob pattern
/// - Invalid patterns will result in a BakerIgnoreError
///
/// # Example
/// ```ignore
/// # Contents of .bakerignore:
/// *.pyc
/// __pycache__/
/// .git/
/// ```
pub fn ignore_file_read<P: AsRef<Path>>(bakerignore_path: P) -> BakerResult<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    if let Ok(contents) = read_to_string(bakerignore_path.as_ref()) {
        for line in contents.lines() {
            builder.add(Glob::new(line).map_err(|e| {
                BakerError::BakerIgnoreError(format!(".bakerignore loading failed: {}", e))
            })?);
        }
    } else {
        debug!(".bakerignore does not exist")
    }
    let glob_set = builder
        .build()
        .map_err(|e| BakerError::BakerIgnoreError(format!(".bakerignore loading failed: {}", e)))?;

    Ok(glob_set)
}
