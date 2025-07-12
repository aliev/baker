use crate::{
    cli::SkipConfirm,
    error::{Error, Result},
    prompt::confirm,
    template::{operation::TemplateOperation, processor::TemplateProcessor},
};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Handles the processing of template files and directories
pub struct FileProcessor<'a> {
    processor: TemplateProcessor<'a, PathBuf>,
    skip_confirms: &'a [SkipConfirm],
    dry_run: bool,
}

impl<'a> FileProcessor<'a> {
    pub fn new(
        processor: TemplateProcessor<'a, PathBuf>,
        skip_confirms: &'a [SkipConfirm],
        dry_run: bool,
    ) -> Self {
        Self { processor, skip_confirms, dry_run }
    }

    /// Processes all files in the template directory
    pub fn process_all_files(&self, template_root: &Path) -> Result<()> {
        for dir_entry in WalkDir::new(template_root) {
            let template_entry = dir_entry?.path().to_path_buf();
            match self.processor.process(template_entry) {
                Ok(file_operation) => {
                    let user_confirmed_overwrite = match &file_operation {
                        TemplateOperation::Ignore { .. } => continue,
                        _ => match self.handle_file_operation(&file_operation) {
                            Ok(confirmed) => confirmed,
                            Err(e) => {
                                log::error!("Failed to handle file operation: {e}");
                                continue;
                            }
                        },
                    };
                    let message = file_operation
                        .get_message(user_confirmed_overwrite, self.dry_run);
                    log::info!("{message}");
                }
                Err(e) => match e {
                    crate::error::Error::ProcessError { .. } => log::warn!("{e}"),
                    _ => log::error!("{e}"),
                },
            }
        }
        Ok(())
    }

    /// Handles a single file operation (write, copy, create directory, or ignore)
    fn handle_file_operation(&self, file_operation: &TemplateOperation) -> Result<bool> {
        log::debug!("Handling file operation: {file_operation:?}");
        match file_operation {
            TemplateOperation::Write { target, target_exists, content, .. } => {
                let skip_prompt = self.should_skip_overwrite_prompt(*target_exists);
                let user_confirmed =
                    confirm(skip_prompt, format!("Overwrite {}?", target.display()))?;

                if user_confirmed {
                    self.write_file(content, target)?;
                }
                Ok(user_confirmed)
            }
            TemplateOperation::Copy { target, target_exists, source, .. } => {
                let skip_prompt = self.should_skip_overwrite_prompt(*target_exists);
                let user_confirmed =
                    confirm(skip_prompt, format!("Overwrite {}?", target.display()))?;

                if user_confirmed {
                    self.copy_file(source, target)?;
                }
                Ok(user_confirmed)
            }
            TemplateOperation::CreateDirectory { target, target_exists } => {
                if !target_exists {
                    self.create_dir_all(target)?;
                }
                Ok(true)
            }
            TemplateOperation::Ignore { .. } => Ok(true),
        }
    }

    /// Copy a file from source to destination, creating parent directories if needed.
    fn copy_file<P: AsRef<Path>>(&self, source_path: P, dest_path: P) -> Result<()> {
        let dest_path = dest_path.as_ref();

        if self.dry_run {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = dest_path.parent() {
            self.create_dir_all(parent)?;
        }

        Ok(std::fs::copy(source_path.as_ref(), dest_path).map(|_| ())?)
    }

    /// Write content to a file, creating parent directories if needed.
    fn write_file<P: AsRef<Path>>(&self, content: &str, dest_path: P) -> Result<()> {
        let dest_path = dest_path.as_ref();

        if self.dry_run {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = dest_path.parent() {
            self.create_dir_all(parent)?;
        }

        std::fs::write(dest_path, content).map_err(Error::from)
    }

    /// Create directory and all parent directories if they don't exist.
    fn create_dir_all<P: AsRef<Path>>(&self, dest_path: P) -> Result<()> {
        if self.dry_run {
            return Ok(());
        }

        std::fs::create_dir_all(dest_path.as_ref()).map_err(Error::from)
    }

    /// Determines if overwrite prompts should be skipped
    fn should_skip_overwrite_prompt(&self, target_exists: bool) -> bool {
        self.skip_confirms.contains(&SkipConfirm::All)
            || self.skip_confirms.contains(&SkipConfirm::Overwrite)
            || !target_exists
    }
}
