use crate::error::{BakerError, BakerResult};
use std::fs;
use std::path::PathBuf;

// Reads bakerfile and returns its content.
pub fn read_bakerfile(bakerfile_path: &PathBuf) -> BakerResult<String> {
    if !bakerfile_path.exists() || !bakerfile_path.is_file() {
        return Err(BakerError::ConfigError(format!(
            "Invalid configuration path: {}",
            bakerfile_path.display()
        )));
    }

    Ok(fs::read_to_string(&bakerfile_path).map_err(BakerError::IoError)?)
}
