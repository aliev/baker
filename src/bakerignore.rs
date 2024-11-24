use crate::error::{BakerError, BakerResult};
use globset::{Glob, GlobSet, GlobSetBuilder};
use log::debug;
use std::{fs::read_to_string, path::PathBuf};

// Fetches the .bakerignore file from template directory and returns GlobSet object.
pub fn read_bakerignore(bakerignore_path: &PathBuf) -> BakerResult<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    if let Ok(contents) = read_to_string(bakerignore_path) {
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
