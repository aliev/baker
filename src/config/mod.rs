//! Configuration management for Baker templates
//!
//! This module contains the configuration system components:
//! - `types`: Basic types and enums used throughout the config system
//! - `question`: Question definition and rendering logic
//! - `loader`: Configuration file loading and parsing

pub mod loader;
pub mod question;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export commonly used types for convenience
pub use loader::{Config, ConfigV1, CONFIG_LIST};
pub use question::{IntoQuestionType, Question, QuestionRendered};
pub use types::{QuestionType, Secret, Type, Validation};
