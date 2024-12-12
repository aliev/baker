use std::io;

use baker::error::Error;

#[test]
fn test_error_conversion() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let baker_err: Error = io_err.into();

    match baker_err {
        Error::IoError(_) => (),
        _ => panic!("Expected IoError variant"),
    }
}

#[test]
fn test_error_display() {
    let err = Error::ConfigError("invalid config".to_string());
    assert_eq!(err.to_string(), "Configuration error: invalid config.");

    let err = Error::TemplateError("rendering failed".to_string());
    assert_eq!(err.to_string(), "Template error: rendering failed.");
}
