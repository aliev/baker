//! Configuration handling for Baker templates.
//! This module provides functionality for loading and processing template configuration files
//! with support for variable interpolation.

use crate::error::{Error, Result};

use dialoguer::Confirm;

pub fn confirm(skip: bool, prompt: String) -> Result<bool> {
    if skip {
        return Ok(true);
    }
    Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()
        .map_err(Error::PromptError)
}
