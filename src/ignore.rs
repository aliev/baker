use crate::{error::Result, ioutils::path_to_str};
use globset::{Glob, GlobSet, GlobSetBuilder};
use log;
use log::{debug, info};
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
pub fn parse_bakerignore_file<P: AsRef<Path>>(template_root: P) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    let template_root = template_root.as_ref();
    let bakerignore_path = template_root.join(IGNORE_FILE);

    // Add default patterns first
    let mut patterns: Vec<String> = DEFAULT_IGNORE_PATTERNS
        .iter()
        .map(|pattern| {
            let path_to_ignored_pattern = template_root.join(pattern);
            path_to_str(&path_to_ignored_pattern).unwrap().to_string()
        })
        .collect();

    // Then add patterns from .bakerignore if it exists
    if let Ok(contents) = read_to_string(bakerignore_path) {
        let ignored_patterns: Vec<String> = contents
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let path_to_ignored_pattern = template_root.join(line);
                path_to_str(&path_to_ignored_pattern).unwrap().to_string()
            })
            .collect();
        patterns.extend(ignored_patterns);
    } else {
        debug!("No .bakerignore file found, using default patterns.");
    }

    for pattern in &patterns {
        debug!("Adding ignore pattern: {pattern} to globset");
        builder.add(Glob::new(pattern)?);
    }
    info!("Loaded the following ignore patterns from .bakerignore file: {patterns:?}");
    Ok(builder.build()?)
}
