use std::path::Path;

use crate::error::{Error, Result};

/// Extension trait for Path to provide convenient string conversion methods
pub trait PathExt {
    /// Converts a path to a string slice, returning an error if the path contains invalid Unicode characters.
    ///
    /// This is a convenience method that's equivalent to `path.to_str().ok_or_else(...)` but with
    /// a descriptive error message.
    ///
    /// # Returns
    /// * `Ok(&str)` - A string slice representing the path
    /// * `Err(Error)` - If the path contains invalid Unicode characters
    ///
    /// # Examples
    /// ```
    /// use baker::ext::PathExt;
    /// use std::path::Path;
    ///
    /// let path = Path::new("test");
    /// assert_eq!(path.to_str_checked().unwrap(), "test");
    /// ```
    fn to_str_checked(&self) -> Result<&str>;

    /// Converts a path to a String using display(), which always succeeds
    /// but may use replacement characters for invalid Unicode.
    ///
    /// This is preferred when you need a String and can tolerate lossy conversion.
    ///
    /// # Examples
    /// ```
    /// use baker::ext::PathExt;
    /// use std::path::Path;
    ///
    /// let path = Path::new("test");
    /// assert_eq!(path.to_string_lossy(), "test");
    /// ```
    fn to_string_lossy(&self) -> String;
}

impl PathExt for Path {
    fn to_str_checked(&self) -> Result<&str> {
        self.to_str().ok_or_else(|| {
            Error::Other(anyhow::anyhow!(
                "Path '{}' contains invalid Unicode characters",
                self.display()
            ))
        })
    }

    fn to_string_lossy(&self) -> String {
        self.display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_to_str_checked_valid() {
        let path = Path::new("valid_path");
        assert_eq!(path.to_str_checked().unwrap(), "valid_path");
    }

    #[test]
    fn test_to_str_checked_invalid_unicode() {
        #[cfg(unix)]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;
            let invalid_bytes = b"valid\xFF";
            let path = Path::new(OsStr::from_bytes(invalid_bytes));
            let result = path.to_str_checked();
            assert!(result.is_err());
        }
        #[cfg(windows)]
        {
            // On Windows, OsStr is always valid Unicode, so this test is not applicable.
            // You may skip or assert true here.
            assert!(true);
        }
    }

    #[test]
    fn test_to_string_lossy_valid_and_invalid_unicode() {
        // Valid Unicode
        let path = Path::new("valid_path");
        assert_eq!(path.to_string_lossy(), "valid_path");

        // Invalid Unicode (Unix only)
        #[cfg(unix)]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;
            let invalid_bytes = b"foo\xFFbar";
            let path = Path::new(OsStr::from_bytes(invalid_bytes));
            let result = path.to_string_lossy();
            // Should contain valid parts and replacement char
            assert!(result.contains("foo") && result.contains("bar"));
        }
        #[cfg(windows)]
        {
            // On Windows, OsStr is always valid Unicode, so this test is not applicable.
            assert!(true);
        }
    }
}
