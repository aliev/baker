use crate::error::{BakerError, BakerResult};
use std::fs;
use std::path::Path;

// Reads bakerfile and returns its content.
pub fn read_bakerfile<P: AsRef<Path>>(bakerfile_path: P) -> BakerResult<String> {
    let bakerfile_path = bakerfile_path.as_ref();
    if !bakerfile_path.exists() || !bakerfile_path.is_file() {
        return Err(BakerError::ConfigError(format!(
            "Invalid configuration path: {}",
            bakerfile_path.display()
        )));
    }

    Ok(fs::read_to_string(bakerfile_path).map_err(BakerError::IoError)?)
}
