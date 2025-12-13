use crate::{
    cli::{context::GenerationContext, SkipConfirm},
    error::{Error, Result},
    prompt::confirm,
    template::{
        operation::{TemplateOperation, WriteOp},
        processor::TemplateProcessor,
    },
};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Handles the processing of template files and directories
pub struct FileProcessor<'a> {
    processor: TemplateProcessor<'a, PathBuf>,
    context: &'a GenerationContext,
}

impl<'a> FileProcessor<'a> {
    pub fn new(
        processor: TemplateProcessor<'a, PathBuf>,
        context: &'a GenerationContext,
    ) -> Self {
        Self { processor, context }
    }

    /// Processes all files in the template directory
    pub fn process_all_files(&self) -> Result<()> {
        for dir_entry in WalkDir::new(self.context.template_root()) {
            let template_entry = dir_entry?.path().to_path_buf();
            let template_name = self.get_template_name(&template_entry);
            match self.processor.process(template_entry) {
                Ok(file_operation) => {
                    let user_confirmed_overwrite = match &file_operation {
                        TemplateOperation::Ignore { .. } => continue,
                        _ => match self.handle_file_operation(&file_operation) {
                            Ok(confirmed) => confirmed,
                            Err(e) => {
                                log::error!(
                                    "Failed to handle file operation for template '{}' ({}): {e}",
                                    template_name,
                                    file_operation.error_context()
                                );
                                continue;
                            }
                        },
                    };
                    let message = file_operation
                        .get_message(user_confirmed_overwrite, self.context.dry_run());
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

    /// Returns the relative path from template root for use in error messages.
    fn get_template_name(&self, path: &Path) -> String {
        path.strip_prefix(self.context.template_root())
            .ok()
            .and_then(|p| p.to_str())
            .map(|s| s.replace('\\', "/"))
            .unwrap_or_else(|| path.display().to_string().replace('\\', "/"))
    }

    /// Handles a single file operation (write, copy, create directory, or ignore)
    fn handle_file_operation(&self, file_operation: &TemplateOperation) -> Result<bool> {
        log::debug!("Handling file operation: {file_operation:?}");
        match file_operation {
            TemplateOperation::Write { target, target_exists, content, .. } => {
                self.handle_write(target, *target_exists, content)
            }
            TemplateOperation::Copy { target, target_exists, source, .. } => {
                self.handle_copy(source, target, *target_exists)
            }
            TemplateOperation::CreateDirectory { target, target_exists } => {
                self.handle_create_dir(target, *target_exists)
            }
            TemplateOperation::Ignore { .. } => Ok(true),
            TemplateOperation::MultipleWrite { writes, .. } => {
                self.handle_multiple_write(writes)
            }
        }
    }

    fn handle_write(
        &self,
        target: &Path,
        target_exists: bool,
        content: &str,
    ) -> Result<bool> {
        let user_confirmed = self.confirm_overwrite(target, target_exists)?;
        if user_confirmed {
            self.write_file(content, target)?;
        }
        Ok(user_confirmed)
    }

    fn handle_copy(
        &self,
        source: &Path,
        target: &Path,
        target_exists: bool,
    ) -> Result<bool> {
        let user_confirmed = self.confirm_overwrite(target, target_exists)?;
        if user_confirmed {
            self.copy_file(source, target)?;
        }
        Ok(user_confirmed)
    }

    fn handle_create_dir(&self, target: &Path, target_exists: bool) -> Result<bool> {
        if !target_exists {
            self.create_dir_all(target)?;
        }
        Ok(true)
    }

    fn handle_multiple_write(&self, writes: &[WriteOp]) -> Result<bool> {
        for write in writes {
            let user_confirmed =
                self.confirm_overwrite(&write.target, write.target_exists)?;
            if user_confirmed {
                self.write_file(&write.content, &write.target)?;
            }
        }
        Ok(true)
    }

    fn confirm_overwrite(&self, target: &Path, target_exists: bool) -> Result<bool> {
        let skip_prompt = self.should_skip_overwrite_prompt(target_exists);
        confirm(skip_prompt, format!("Overwrite {}?", target.display()))
    }

    /// Copy a file from source to destination, creating parent directories if needed.
    fn copy_file<P: AsRef<Path>>(&self, source_path: P, dest_path: P) -> Result<()> {
        let dest_path = dest_path.as_ref();
        let source_path = source_path.as_ref();

        if self.context.dry_run() {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = dest_path.parent() {
            self.create_dir_all(parent)?;
        }

        let metadata = std::fs::symlink_metadata(source_path)?;
        if metadata.file_type().is_symlink() {
            if self.context.config().follow_symlinks {
                return self.copy_followed_symlink(source_path, dest_path);
            } else {
                return self.copy_symlink(source_path, dest_path);
            }
        }

        Ok(std::fs::copy(source_path, dest_path).map(|_| ())?)
    }

    /// When follow_symlinks is enabled, copy the content the symlink points to.
    fn copy_followed_symlink(&self, source_link: &Path, dest_path: &Path) -> Result<()> {
        let target_rel = std::fs::read_link(source_link)?;
        // Resolve relative targets against the symlink's parent directory.
        let resolved_target = if target_rel.is_relative() {
            source_link.parent().unwrap_or_else(|| Path::new("")).join(&target_rel)
        } else {
            target_rel.clone()
        };
        let target_meta = std::fs::metadata(&resolved_target)?;
        if target_meta.is_file() {
            std::fs::copy(&resolved_target, dest_path)?;
            return Ok(());
        }
        // For now, if it's a directory (or other), fall back to recreating symlink.
        self.copy_symlink(source_link, dest_path)
    }

    /// Recreate a symbolic link at destination preserving original (possibly relative) target.
    /// On overwrite, existing file/symlink is removed first.
    fn copy_symlink(&self, source_path: &Path, dest_path: &Path) -> Result<()> {
        let link_target = std::fs::read_link(source_path)?;
        if dest_path.exists() || dest_path.is_symlink() {
            std::fs::remove_file(dest_path)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(&link_target, dest_path)?;
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::{symlink_dir, symlink_file};
            let target_is_dir =
                link_target.canonicalize().map(|p| p.is_dir()).unwrap_or(false);
            if target_is_dir {
                symlink_dir(&link_target, dest_path)?;
            } else {
                symlink_file(&link_target, dest_path)?;
            }
        }
        Ok(())
    }

    /// Write content to a file, creating parent directories if needed.
    fn write_file<P: AsRef<Path>>(&self, content: &str, dest_path: P) -> Result<()> {
        let dest_path = dest_path.as_ref();

        if self.context.dry_run() {
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
        if self.context.dry_run() {
            return Ok(());
        }

        std::fs::create_dir_all(dest_path.as_ref()).map_err(Error::from)
    }

    /// Determines if overwrite prompts should be skipped
    fn should_skip_overwrite_prompt(&self, target_exists: bool) -> bool {
        self.context.skip_confirms().contains(&SkipConfirm::All)
            || self.context.skip_confirms().contains(&SkipConfirm::Overwrite)
            || !target_exists
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::MiniJinjaRenderer;
    use globset::GlobSetBuilder;
    use indexmap::IndexMap;
    use serde_json::json;
    use tempfile::TempDir;

    fn build_file_processor(
        skip_confirms: Vec<SkipConfirm>,
        follow_symlinks: bool,
    ) -> (TempDir, TempDir, FileProcessor<'static>) {
        let template_root = TempDir::new().unwrap();
        let output_root = TempDir::new().unwrap();
        let engine = Box::leak(Box::new(MiniJinjaRenderer::new()));
        let bakerignore = Box::leak(Box::new(GlobSetBuilder::new().build().unwrap()));

        let mut context = GenerationContext::new(
            template_root.path().to_path_buf(),
            output_root.path().to_path_buf(),
            crate::config::ConfigV1 {
                template_suffix: ".baker.j2".into(),
                loop_separator: "".into(),
                loop_content_separator: "".into(),
                template_globs: Vec::new(),
                import_root: None,
                questions: IndexMap::new(),
                post_hook_filename: "post".into(),
                pre_hook_filename: "pre".into(),
                post_hook_runner: Vec::new(),
                pre_hook_runner: Vec::new(),
                follow_symlinks,
            },
            skip_confirms,
            false,
        );
        context.set_answers(json!({}));
        let context = Box::leak(Box::new(context));
        let processor = TemplateProcessor::new(&*engine, context, &*bakerignore);

        (template_root, output_root, FileProcessor::new(processor, context))
    }

    #[test]
    fn skips_overwrite_prompt_for_new_files() {
        let (_template_root, _output_root, processor) =
            build_file_processor(Vec::new(), false);
        assert!(processor.should_skip_overwrite_prompt(false));
        assert!(!processor.should_skip_overwrite_prompt(true));
    }

    #[test]
    fn skips_overwrite_prompt_when_flagged() {
        let (_template_root, _output_root, processor) =
            build_file_processor(vec![SkipConfirm::Overwrite], false);
        assert!(processor.should_skip_overwrite_prompt(true));
    }

    #[test]
    fn confirm_overwrite_short_circuits_when_skip_applies() {
        let (_template_root, output_root, processor) =
            build_file_processor(Vec::new(), false);
        let target = output_root.path().join("new-file.txt");
        let result = processor.confirm_overwrite(&target, false).unwrap();
        assert!(result);
    }

    #[test]
    #[cfg(unix)]
    fn copies_symlink_preserves_relative_target() {
        use std::os::unix::fs::symlink;
        let (template_root, output_root, processor) =
            build_file_processor(Vec::new(), false);
        let src_file = template_root.path().join("orig.txt");
        std::fs::write(&src_file, "hello").unwrap();
        let src_link = template_root.path().join("link.txt");
        symlink("orig.txt", &src_link).unwrap(); // relative target
        let dest_link = output_root.path().join("link.txt");
        processor.copy_file(&src_link, &dest_link).unwrap();
        assert!(dest_link.is_symlink());
        let recreated_target = std::fs::read_link(dest_link).unwrap();
        assert_eq!(recreated_target, PathBuf::from("orig.txt"));
    }

    #[test]
    #[cfg(windows)]
    fn copies_symlink_preserves_relative_target() {
        use std::os::windows::fs::symlink_file;
        let (template_root, output_root, processor) =
            build_file_processor(Vec::new(), false);
        let src_file = template_root.path().join("orig.txt");
        std::fs::write(&src_file, "hello").unwrap();
        let src_link = template_root.path().join("link.txt");
        symlink_file(&src_file, &src_link).unwrap();
        let dest_link = output_root.path().join("link.txt");
        processor.copy_file(&src_link, &dest_link).unwrap();
        assert!(dest_link.is_symlink());
    }

    #[test]
    fn get_template_name_returns_relative_path() {
        let (template_root, _output_root, processor) =
            build_file_processor(Vec::new(), false);
        let nested_path =
            template_root.path().join("subdir").join("nested").join("file.txt");
        let result = processor.get_template_name(&nested_path);
        assert_eq!(result, "subdir/nested/file.txt");
    }

    #[test]
    fn get_template_name_returns_filename_at_root() {
        let (template_root, _output_root, processor) =
            build_file_processor(Vec::new(), false);
        let file_path = template_root.path().join("file.txt");
        let result = processor.get_template_name(&file_path);
        assert_eq!(result, "file.txt");
    }

    #[test]
    fn get_template_name_returns_full_path_for_unrelated() {
        let (_template_root, _output_root, processor) =
            build_file_processor(Vec::new(), false);
        let unrelated_path = PathBuf::from("/some/other/path/file.txt");
        let result = processor.get_template_name(&unrelated_path);
        assert_eq!(result, "/some/other/path/file.txt");
    }

    #[test]
    #[cfg(unix)]
    fn copies_followed_symlink_creates_regular_file() {
        use std::os::unix::fs::symlink;
        let (template_root, output_root, processor) =
            build_file_processor(Vec::new(), true);
        let src_file = template_root.path().join("orig.txt");
        std::fs::write(&src_file, "hello-follow").unwrap();
        let src_link = template_root.path().join("link.txt");
        symlink("orig.txt", &src_link).unwrap();
        let dest_link = output_root.path().join("link.txt");
        processor.copy_file(&src_link, &dest_link).unwrap();
        assert!(!dest_link.is_symlink());
        assert_eq!(std::fs::read_to_string(dest_link).unwrap(), "hello-follow");
    }
}
