//! File and directory ignore pattern handling for Baker templates.
//! This module processes .bakerignore files to exclude specific paths
//! from template processing, similar to .gitignore functionality.

use crate::error::{Error, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use log::debug;
use std::{fs::read_to_string, path::Path};

/// Default patterns to always ignore during template processing
const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    ".git/**",
    ".git",
    ".hg/**",
    ".hg",
    ".svn/**",
    ".svn",
    "**/.DS_Store",
    ".bakerignore",
    "hooks",
    "hooks/**",
    "baker.yaml",
    "baker.yml",
    "baker.json",
];

/// Baker's ignore file name
pub const IGNORE_FILE: &str = ".bakerignore";

/// Reads and processes the .bakerignore file to create a set of glob patterns.
///
/// # Arguments
/// * `bakerignore_path` - Path to the ignore file (typically .bakerignore)
///
/// # Returns
/// * `BakerResult<GlobSet>` - Set of compiled glob patterns for path matching
///
/// # Notes
/// - If the ignore file doesn't exist, returns an empty GlobSet
/// - Each line in the file is treated as a separate glob pattern
/// - Invalid patterns will result in a BakerIgnoreError
pub fn parse_bakerignore_file<P: AsRef<Path>>(template_root: P) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    let template_root = template_root.as_ref();
    let bakerignore_path = template_root.join(IGNORE_FILE);

    // Add default patterns first
    for pattern in DEFAULT_IGNORE_PATTERNS {
        builder.add(
            Glob::new(template_root.join(pattern).to_str().unwrap())
                .map_err(Error::GlobSetParseError)?,
        );
    }

    // Then add patterns from .bakerignore if it exists
    if let Ok(contents) = read_to_string(bakerignore_path) {
        for line in contents.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                builder.add(
                    Glob::new(template_root.join(line).to_str().unwrap())
                        .map_err(Error::GlobSetParseError)?,
                );
            }
        }
    } else {
        debug!("No .bakerignore file found, using default patterns.");
    }

    builder.build().map_err(Error::GlobSetParseError)
}
