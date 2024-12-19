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
enum FileAction {
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

    fn determine_file_action(
        &self,
        target_path_exists: bool,
        user_confirmed_overwrite: bool,
    ) -> FileAction {
        match (target_path_exists, user_confirmed_overwrite) {
            (true, true) => FileAction::Overwrite,
            (false, _) => FileAction::Create,
            (true, false) => FileAction::Skip,
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

        for (template_path, rendered_path) in
            template_path.iter().zip(rendered_path.iter())
        {
            if self.is_template_string(template_path) && rendered_path.is_empty() {
                return false;
            }
        }

        true
    }

    fn write_file<T: AsRef<Path>>(&self, target_path: T, content: &str) -> Result<()> {
        let target_path = target_path.as_ref();
        let base_path = std::env::current_dir().unwrap_or_default();
        let abs_path = if target_path.is_absolute() {
            target_path.to_path_buf()
        } else {
            base_path.join(target_path)
        };

        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent).map_err(Error::IoError)?;
        }
        fs::write(abs_path, content).map_err(Error::IoError)
    }

    fn copy_file<T: AsRef<Path>>(&self, source_path: T, dest_path: T) -> Result<()> {
        let dest_path = dest_path.as_ref();
        let base_path = std::env::current_dir().unwrap_or_default();
        let abs_dest = if dest_path.is_absolute() {
            dest_path.to_path_buf()
        } else {
            base_path.join(dest_path)
        };

        if let Some(parent) = abs_dest.parent() {
            fs::create_dir_all(parent).map_err(Error::IoError)?;
        }
        fs::copy(source_path, abs_dest).map(|_| ()).map_err(Error::IoError)
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

    fn is_template_string(&self, text: &str) -> bool {
        text.starts_with("{%") && text.ends_with("%}")
            || text.starts_with("{{") && text.ends_with("}}")
    }

    fn render_path<T: AsRef<Path>>(&self, template_path: T) -> Result<String> {
        let template_path = template_path.as_ref();
        let path_str = template_path.to_str().ok_or_else(|| Error::ProcessError {
            source_path: template_path.display().to_string(),
            e: "Cannot convert source_path to string.".to_string(),
        })?;

        self.engine.render(path_str, self.answers).map_err(|e| Error::ProcessError {
            source_path: path_str.to_string(),
            e: e.to_string(),
        })
    }

    fn render_content<T: AsRef<Path>>(&self, template_path: T) -> Result<String> {
        let template_path = template_path.as_ref();
        let content = fs::read_to_string(template_path).map_err(Error::IoError)?;
        self.engine.render(&content, self.answers)
    }

    pub fn process(&self, template_entry: P) -> Result<()> {
        let template_entry = template_entry.as_ref();

        if self.ignored_patterns.is_match(template_entry) {
            // TODO: Return the process result
            println!("Skipping (.bakerignore): '{}'.", template_entry.display());
            return Ok(());
        }

        let rendered_entry = self.render_path(template_entry)?;
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
        let rendered_content = self.render_content(template_entry)?;

        let prompt_not_needed = self.skip_overwrite_check || !target_path.exists();
        let user_confirmed_overwrite = self.prompt.confirm(
            prompt_not_needed,
            format!("Overwrite {}?", target_path.display()),
        )?;

        let action =
            self.determine_file_action(target_path.exists(), user_confirmed_overwrite);

        println!("{}: '{}'", action, target_path.display());

        match action {
            // TODO: Return the process result
            FileAction::Create | FileAction::Overwrite => {
                // TODO: Move to run
                if self.is_template_file(template_entry) {
                    self.copy_file(template_entry, &target_path)?;
                } else {
                    self.write_file(target_path, &rendered_content)?;
                }
                // return Process
                Ok(())
            }
            // Return Skip
            FileAction::Skip => Ok(()),
        }
    }
}
