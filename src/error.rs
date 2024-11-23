use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BakerError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Template error: {0}")]
    TemplateError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Hook execution error: {0}")]
    HookError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("BakerIgnore error: {0}")]
    BakerIgnoreError(String),
}

pub type BakerResult<T> = Result<T, BakerError>;
