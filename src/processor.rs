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
    CopyFile { target: PathBuf },
    CreateDir { target: PathBuf },
    WriteFile { target: PathBuf, content: String },
}
#[derive(Debug)]
pub struct ProcessResult {
    pub action: FileAction,
    pub operation: Option<FileOperation>,
    pub source: PathBuf,
}

pub struct Processor<'a, P: AsRef<Path>> {
    /// Dependencies
    engine: &'a dyn TemplateRenderer,
    prompt: &'a dyn Prompter,
    bakerignore: &'a GlobSet,

    /// Other
    template_root: P,
    output_root: P,
    skip_overwrite_check: bool,
    answers: &'a serde_json::Value,
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
            bakerignore: ignored_patterns,
        }
    }

    fn has_valid_rendered_path_parts<S: Into<String>>(
        &self,
        template_path: S,
        rendered_path: S,
    ) -> bool {
        let template_path = template_path.into();
        let rendered_path = rendered_path.into();
        let template_path: Vec<&str> =
            template_path.split(std::path::MAIN_SEPARATOR).collect();
        let rendered_path: Vec<&str> =
            rendered_path.split(std::path::MAIN_SEPARATOR).collect();

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
        // The `template_entry` refers to any file or directory within the `template_root`.
        let template_entry = template_entry.as_ref();

        // Check if the `template_entry` is listed in the `.bakerignore` file.
        // If it is, stop processing and return a skip action.
        if self.bakerignore.is_match(template_entry) {
            return Ok(ProcessResult {
                source: template_entry.to_path_buf(),
                action: FileAction::Skip,
                operation: None,
            });
        }

        // The `template_entry` file or directory name may contain a template string.
        // This allows the system to determine whether to create a file or directory and
        // how to resolve its name based on provided template data.
        //
        // For example, if the file or directory name contains the string:
        // `{{filename}}.txt`, it will be rendered as `my_file_name.txt`
        // if the value for "filename" in `self.answers` is "my_file_name".
        //
        // Additionally, conditions can be applied. For instance, if the file or directory name
        // has an empty value, it will not be created.
        // Example: `{% if create_tests %}tests{% endif %}/` will create the directory only
        // if `create_tests` in `self.answers` evaluates to true.
        let rendered_entry = self.engine.render_path(template_entry, self.answers)?;
        let rendered_entry = rendered_entry.as_str();

        // Validates whether the `rendered_entry` is properly rendered by comparing its components
        // with those of the original `template_entry`. The validation ensures no parts of the path
        // are empty after rendering.
        //
        // Example:
        // Given the following `template_entry`:
        // `template_root/{% if create_tests %}tests{% endif %}/`
        // And a corresponding `rendered_entry`:
        // `template_root/tests/`
        //
        // The `has_valid_rendered_path_parts` function splits both paths by "/" and compares
        // their parts. If none of the parts are empty, the function concludes that the path
        // was correctly rendered and proceeds with processing.
        //
        // However, if the `create_tests` value in `self.answers` is `false`, the rendered path
        // will look like this:
        // `template_root//`
        //
        // When compared with the original `template_entry`, `template_root/{% if create_tests %}tests{% endif %}/`,
        // the function detects that one of the parts is empty (due to the double "//").
        // In such cases, it considers the rendered path invalid and skips further processing.
        //
        if !self.has_valid_rendered_path_parts(
            template_entry.to_str().unwrap_or_default(),
            rendered_entry,
        ) {
            return Err(Error::ProcessError {
                source_path: rendered_entry.to_string(),
                e: "The rendered path is not valid".to_string(),
            });
        }

        // Removes the `.j2` suffix to create the target filename with its actual extension.
        //
        // The following lines check whether the `template_entry` is a template file by
        // determining if its filename ends with a double extension that includes `.j2`.
        // For example:
        // - `README.md.j2` will be considered a template file because it has the double
        //   extensions `.md` and `.j2`.
        // - `.dockerignore.j2` will also be considered a template file since it includes
        //   `.dockerignore` and `.j2` as extensions.
        //
        // However, filenames like `template.j2` or `README.md` will not be considered
        // template files because they lack a double extension with `.j2`.
        //
        let rendered_entry = if self.is_template_file(template_entry) {
            rendered_entry.strip_suffix(".j2").unwrap_or_default()
        } else {
            rendered_entry
        };

        // Converts the `rendered_entry` slice to a `PathBuf` for easier manipulation
        // in subsequent operations.
        let rendered_path_buf = PathBuf::from(rendered_entry);

        // Constructs the `target_path` from `rendered_path_buf`, which represents the
        // actual path to the file or directory that will be created in `output_root`.
        //
        // The `target_path` is built by replacing the `template_root` prefix with the `output_root` prefix.
        // Example:
        // If `rendered_path_buf` is:
        // `PathBuf("template_root/tests/__init__.py")`
        //
        // The `template_root` prefix is replaced with `output_root`, resulting in:
        // `PathBuf("output_root/tests/__init__.py")`
        //
        // Here, `output_root` is the directory where the rendered file or directory will be saved.
        //
        let target_path = rendered_path_buf
            .strip_prefix(self.template_root.as_ref())
            .map_err(|e| Error::ProcessError {
                source_path: template_entry.display().to_string(),
                e: e.to_string(),
            })?;
        let target_path = self.output_root.as_ref().join(target_path);

        // If the `target_path` exists and is a directory,
        // skip it from further processing.
        if target_path.exists() && target_path.is_dir() {
            return Ok(ProcessResult {
                action: FileAction::Skip,
                operation: None,
                source: template_entry.to_path_buf(),
            });
        }

        // Determines whether to prompt the user for overwrite confirmation:
        // - Skips the prompt if `self.skip_overwrite_check` is true or if the `target_path` does not exist.
        // - Otherwise, prompts the user with a confirmation message: "Overwrite <target_path>?"
        //
        let skip_user_ask = self.skip_overwrite_check || !target_path.exists();
        let user_confirmed_overwrite = self
            .prompt
            .confirm(skip_user_ask, format!("Overwrite {}?", target_path.display()))?;

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

        let operation = if template_entry.is_file()
            && self.is_template_file(template_entry)
        {
            // If `template_entry` is a file and a template file, read its content and
            // process it using the template engine with `self.answers` as the context.
            let template_content =
                fs::read_to_string(template_entry).map_err(Error::IoError)?;
            let rendered_content = self.engine.render(&template_content, self.answers)?;
            FileOperation::WriteFile { target: target_path, content: rendered_content }
        } else if template_entry.is_dir() {
            // If `template_entry` is a directory, create the corresponding target directory.
            FileOperation::CreateDir { target: target_path }
        } else {
            // Otherwise, copy the source file to the target path as-is.
            FileOperation::CopyFile { target: target_path }
        };

        Ok(ProcessResult {
            action,
            operation: Some(operation),
            source: template_entry.to_path_buf(),
        })
    }
}
