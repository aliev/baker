use std::path::Path;

use crate::error::{Error, Result};

/// Converts a path to a string slice, returning an error if the path contains invalid Unicode characters.
///
/// # Arguments
/// * `path` - A reference to a type that can be converted to a [`Path`]
///
/// # Returns
/// * `Ok(&str)` - A string slice representing the path
/// * `Err(Error)` - If the path contains invalid Unicode characters
///
/// # Examples
/// ```
/// use baker::ioutils::path_to_str;
/// use std::path::Path;
///
/// let path = Path::new("test");
/// assert_eq!(path_to_str(path).unwrap(), "test");
///
/// // Invalid paths will return an error
/// #[cfg(unix)]
/// {
///     use std::os::unix::ffi::OsStrExt;
///     use std::ffi::OsStr;
///     let invalid_bytes = [0x80, 0x00];
///     let invalid_path = Path::new(OsStr::from_bytes(&invalid_bytes));
///     assert!(path_to_str(invalid_path).is_err());
/// }
/// ```
pub fn path_to_str<P: AsRef<Path> + ?Sized>(path: &P) -> Result<&str> {
    path.as_ref().to_str().ok_or_else(|| {
        Error::Other(anyhow::anyhow!(
            "Path '{}' contains invalid Unicode characters",
            path.as_ref().display()
        ))
    })
}
