//! Core template processing module for Baker.
//! Handles file system operations, template rendering, and output generation
//! with support for path manipulation and error handling.
use globset::GlobSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::prompt::Prompter;
use crate::renderer::TemplateRenderer;

#[derive(Debug)]
pub enum FileAction {
    Create,
    Overwrite,
    Skip,
}

impl std::fmt::Display for FileAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileAction::Create => write!(f, "Creating"),
            FileAction::Overwrite => write!(f, "Overwriting"),
            FileAction::Skip => write!(f, "Skipping"),
        }
    }
}

#[derive(Debug)]
pub enum FileOperation {
    Copy { target: PathBuf },
    Write { target: PathBuf, content: String },
}

pub struct ProcessResult {
    pub action: FileAction,
    pub operation: Option<FileOperation>,
    pub source: PathBuf,
}

pub struct Processor<'a, P: AsRef<Path>> {
    engine: &'a dyn TemplateRenderer,
    prompt: &'a dyn Prompter,
    template_root: P,
    output_root: P,
    skip_overwrite_check: bool,
    answers: &'a serde_json::Value,
    ignored_patterns: &'a GlobSet,
}

impl<'a, P: AsRef<Path>> Processor<'a, P> {
    pub fn new(
        engine: &'a dyn TemplateRenderer,
        prompt: &'a dyn Prompter,
        template_root: P,
        output_root: P,
        skip_overwrite_check: bool,
        answers: &'a serde_json::Value,
        ignored_patterns: &'a GlobSet,
    ) -> Self {
        Self {
            engine,
            prompt,
            template_root,
            output_root,
            skip_overwrite_check,
            answers,
            ignored_patterns,
        }
    }

    fn has_valid_rendered_path_parts<S: Into<String>>(
        &self,
        template_path: S,
        rendered_path: S,
    ) -> bool {
        let template_path = template_path.into();
        let rendered_path = rendered_path.into();
        let template_path: Vec<&str> = template_path.split('/').collect();
        let rendered_path: Vec<&str> = rendered_path.split('/').collect();

        for (template_part, rendered_part) in
            template_path.iter().zip(rendered_path.iter())
        {
            if !template_part.is_empty() && rendered_part.is_empty() {
                return false;
            }
        }

        true
    }

    fn is_template_file<T: AsRef<Path>>(&self, path: T) -> bool {
        let path = path.as_ref();
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => return false,
        };

        let parts: Vec<&str> = file_name.split('.').collect();
        parts.len() > 2 && parts.last() == Some(&"j2")
    }

    pub fn process(&self, template_entry: P) -> Result<ProcessResult> {
        let template_entry = template_entry.as_ref();

        if self.ignored_patterns.is_match(template_entry) {
            return Ok(ProcessResult {
                source: template_entry.to_path_buf(),
                action: FileAction::Skip,
                operation: None,
            });
        }

        let rendered_entry = self.engine.render_path(template_entry, self.answers)?;
        let rendered_entry = rendered_entry.as_str();

        if !self.has_valid_rendered_path_parts(
            template_entry.to_str().unwrap_or_default(),
            rendered_entry,
        ) {
            return Err(Error::ProcessError {
                source_path: rendered_entry.to_string(),
                e: "The rendered path is not valid".to_string(),
            });
        }

        let rendered_path = if self.is_template_file(rendered_entry) {
            rendered_entry.strip_suffix(".j2").unwrap_or_default()
        } else {
            rendered_entry
        };

        let rendered_path_buf = PathBuf::from(rendered_path);
        let target_path = rendered_path_buf
            .strip_prefix(self.template_root.as_ref())
            .map_err(|e| Error::ProcessError {
                source_path: template_entry.display().to_string(),
                e: e.to_string(),
            })?;

        let target_path = self.output_root.as_ref().join(target_path);
        let template_content =
            fs::read_to_string(template_entry).map_err(Error::IoError)?;
        let rendered_content = self.engine.render(&template_content, self.answers)?;

        let prompt_not_needed = self.skip_overwrite_check || !target_path.exists();
        let user_confirmed_overwrite = self.prompt.confirm(
            prompt_not_needed,
            format!("Overwrite {}?", target_path.display()),
        )?;

        let action = match (target_path.exists(), user_confirmed_overwrite) {
            (true, true) => FileAction::Overwrite,
            (false, _) => FileAction::Create,
            (true, false) => {
                return Ok(ProcessResult {
                    action: FileAction::Skip,
                    operation: None,
                    source: template_entry.to_path_buf(),
                })
            }
        };

        let operation = if self.is_template_file(template_entry) {
            FileOperation::Write { target: target_path, content: rendered_content }
        } else {
            FileOperation::Copy { target: target_path }
        };

        Ok(ProcessResult {
            action,
            operation: Some(operation),
            source: template_entry.to_path_buf(),
        })
    }
}
