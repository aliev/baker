use std::path::PathBuf;

#[derive(Debug)]
pub enum TemplateOperation {
    Copy { source: PathBuf, target: PathBuf, target_exists: bool },
    Write { target: PathBuf, content: String, target_exists: bool },
    CreateDirectory { target: PathBuf, target_exists: bool },
    Ignore { source: PathBuf },
    MultipleWrite { writes: Vec<WriteOp> },
}

#[derive(Debug)]
pub struct WriteOp {
    pub target: PathBuf,
    pub content: String,
    pub target_exists: bool,
}

impl TemplateOperation {
    /// Returns the target path for this operation, used for error context.
    ///
    /// # Returns
    /// * `Option<&PathBuf>` - The target path, or None for operations without a single target
    pub fn target_path(&self) -> Option<&PathBuf> {
        match self {
            TemplateOperation::Copy { target, .. } => Some(target),
            TemplateOperation::Write { target, .. } => Some(target),
            TemplateOperation::CreateDirectory { target, .. } => Some(target),
            TemplateOperation::Ignore { .. } => None,
            TemplateOperation::MultipleWrite { .. } => None,
        }
    }

    /// Returns a brief description of this operation for error messages.
    ///
    /// # Returns
    /// * `String` - A concise description including the operation type and paths
    pub fn error_context(&self) -> String {
        match self {
            TemplateOperation::Copy { source, target, .. } => {
                format!("copy '{}' -> '{}'", source.display(), target.display())
            }
            TemplateOperation::Write { target, .. } => {
                format!("write '{}'", target.display())
            }
            TemplateOperation::CreateDirectory { target, .. } => {
                format!("create directory '{}'", target.display())
            }
            TemplateOperation::Ignore { source } => {
                format!("ignore '{}'", source.display())
            }
            TemplateOperation::MultipleWrite { writes } => {
                let targets: Vec<_> =
                    writes.iter().map(|w| w.target.display().to_string()).collect();
                format!("write multiple files: {}", targets.join(", "))
            }
        }
    }

    /// Gets a message describing the operation and its status.
    ///
    /// # Arguments
    /// * `user_confirmed_overwrite` - Whether the user has confirmed overwriting existing files
    /// * `dry_run` - Whether this is a dry run (no actual file operations)
    ///
    /// # Returns
    /// * `String` - A descriptive message about the operation
    pub fn get_message(&self, user_confirmed_overwrite: bool, dry_run: bool) -> String {
        let prefix = if dry_run { "[DRY RUN] " } else { "" };

        match self {
            TemplateOperation::Copy { source, target, target_exists } => {
                if *target_exists {
                    if user_confirmed_overwrite {
                        format!(
                            "{}Copying '{}' to '{}' (overwriting existing file)",
                            prefix,
                            source.display(),
                            target.display()
                        )
                    } else {
                        format!(
                            "{}Skipping copy of '{}' to '{}' (target already exists)",
                            prefix,
                            source.display(),
                            target.display()
                        )
                    }
                } else {
                    format!(
                        "{}Copying '{}' to '{}'",
                        prefix,
                        source.display(),
                        target.display()
                    )
                }
            }

            TemplateOperation::CreateDirectory { target, target_exists } => {
                if *target_exists {
                    format!(
                        "{}Skipping directory creation '{}' (already exists)",
                        prefix,
                        target.display()
                    )
                } else {
                    format!("{}Creating directory '{}'", prefix, target.display())
                }
            }

            TemplateOperation::Write { target, target_exists, .. } => {
                if *target_exists {
                    if user_confirmed_overwrite {
                        format!(
                            "{}Writing to '{}' (overwriting existing file)",
                            prefix,
                            target.display()
                        )
                    } else {
                        format!(
                            "{}Skipping write to '{}' (target already exists)",
                            prefix,
                            target.display()
                        )
                    }
                } else {
                    format!("{}Writing to '{}'", prefix, target.display())
                }
            }

            TemplateOperation::Ignore { source } => {
                format!(
                    "{}Ignoring '{}' (matches ignore pattern)",
                    prefix,
                    source.display()
                )
            }

            TemplateOperation::MultipleWrite { writes } => {
                let overwriting: Vec<_> = writes
                    .iter()
                    .filter(|w| w.target_exists)
                    .map(|w| w.target.display().to_string())
                    .collect();
                let writing: Vec<_> = writes
                    .iter()
                    .filter(|w| !w.target_exists)
                    .map(|w| w.target.display().to_string())
                    .collect();
                let mut msg = String::new();
                if !overwriting.is_empty() {
                    msg.push_str(&format!("Overwriting: {}.", overwriting.join(", ")));
                }
                if !writing.is_empty() {
                    msg.push_str(&format!("Writing: {}.", writing.join(", ")));
                }
                msg.trim_end().to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_operation_logs_overwrite_message() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = true;
        let expected = format!(
            "Copying '{}' to '{}' (overwriting existing file)",
            &source.display(),
            &target.display()
        );

        let copy = TemplateOperation::Copy { source, target, target_exists: true };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }

    #[test]
    fn copy_operation_skips_when_not_confirmed() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected = format!(
            "Skipping copy of '{}' to '{}' (target already exists)",
            &source.display(),
            &target.display()
        );

        let copy = TemplateOperation::Copy { source, target, target_exists: true };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }

    #[test]
    fn copy_operation_logs_basic_message() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected =
            format!("Copying '{}' to '{}'", &source.display(), &target.display());

        let copy = TemplateOperation::Copy { source, target, target_exists: false };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }
    #[test]
    fn create_directory_skips_when_exists() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected = format!(
            "Skipping directory creation '{}' (already exists)",
            &target.display()
        );

        let copy = TemplateOperation::CreateDirectory { target, target_exists: true };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }
    #[test]
    fn create_directory_message_when_missing() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected = format!("Creating directory '{}'", &target.display());

        let copy = TemplateOperation::CreateDirectory { target, target_exists: false };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }
    #[test]
    fn write_operation_overwrite_message() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = true;
        let expected =
            format!("Writing to '{}' (overwriting existing file)", &target.display());

        let copy = TemplateOperation::Write {
            target,
            target_exists: true,
            content: "".to_string(),
        };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }
    #[test]
    fn write_operation_skips_without_confirmation() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected =
            format!("Skipping write to '{}' (target already exists)", &target.display());

        let copy = TemplateOperation::Write {
            target,
            target_exists: true,
            content: "".to_string(),
        };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }
    #[test]
    fn write_operation_basic_message() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected = format!("Writing to '{}'", &target.display());

        let copy = TemplateOperation::Write {
            target,
            target_exists: false,
            content: "".to_string(),
        };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }
    #[test]
    fn ignore_operation_logs_message() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected =
            format!("Ignoring '{}' (matches ignore pattern)", &source.display());

        let copy = TemplateOperation::Ignore { source };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_dry_run_messages() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let target = PathBuf::from("/tmp/test/target.txt");

        let copy = TemplateOperation::Copy { source, target, target_exists: false };
        let dry_run_message = copy.get_message(false, true);
        let normal_message = copy.get_message(false, false);

        assert!(dry_run_message.starts_with("[DRY RUN] "));
        assert!(!normal_message.starts_with("[DRY RUN] "));
        assert_eq!(dry_run_message, format!("[DRY RUN] {}", normal_message));
    }

    #[test]
    fn target_path_returns_target_for_copy() {
        let source = PathBuf::from("/tmp/source.txt");
        let target = PathBuf::from("/tmp/target.txt");
        let op = TemplateOperation::Copy {
            source,
            target: target.clone(),
            target_exists: false,
        };
        assert_eq!(op.target_path(), Some(&target));
    }

    #[test]
    fn target_path_returns_target_for_write() {
        let target = PathBuf::from("/tmp/output.txt");
        let op = TemplateOperation::Write {
            target: target.clone(),
            content: "content".to_string(),
            target_exists: false,
        };
        assert_eq!(op.target_path(), Some(&target));
    }

    #[test]
    fn target_path_returns_target_for_create_directory() {
        let target = PathBuf::from("/tmp/new_dir");
        let op = TemplateOperation::CreateDirectory {
            target: target.clone(),
            target_exists: false,
        };
        assert_eq!(op.target_path(), Some(&target));
    }

    #[test]
    fn target_path_returns_none_for_ignore() {
        let source = PathBuf::from("/tmp/ignored.txt");
        let op = TemplateOperation::Ignore { source };
        assert_eq!(op.target_path(), None);
    }

    #[test]
    fn target_path_returns_none_for_multiple_write() {
        let writes = vec![
            WriteOp {
                target: PathBuf::from("/tmp/file1.txt"),
                content: "content1".to_string(),
                target_exists: false,
            },
            WriteOp {
                target: PathBuf::from("/tmp/file2.txt"),
                content: "content2".to_string(),
                target_exists: false,
            },
        ];
        let op = TemplateOperation::MultipleWrite { writes };
        assert_eq!(op.target_path(), None);
    }

    #[test]
    fn error_context_for_copy_includes_source_and_target() {
        let source = PathBuf::from("/template/src/file.txt");
        let target = PathBuf::from("/output/src/file.txt");
        let op = TemplateOperation::Copy { source, target, target_exists: false };
        let context = op.error_context();
        assert!(context.contains("copy"));
        assert!(context.contains("/template/src/file.txt"));
        assert!(context.contains("/output/src/file.txt"));
    }

    #[test]
    fn error_context_for_write_includes_target() {
        let target = PathBuf::from("/output/deep/nested/file.txt");
        let op = TemplateOperation::Write {
            target,
            content: "content".to_string(),
            target_exists: false,
        };
        let context = op.error_context();
        assert!(context.contains("write"));
        assert!(context.contains("/output/deep/nested/file.txt"));
    }

    #[test]
    fn error_context_for_create_directory_includes_target() {
        let target = PathBuf::from("/output/new/directory");
        let op = TemplateOperation::CreateDirectory { target, target_exists: false };
        let context = op.error_context();
        assert!(context.contains("create directory"));
        assert!(context.contains("/output/new/directory"));
    }

    #[test]
    fn error_context_for_ignore_includes_source() {
        let source = PathBuf::from("/template/ignored/file.txt");
        let op = TemplateOperation::Ignore { source };
        let context = op.error_context();
        assert!(context.contains("ignore"));
        assert!(context.contains("/template/ignored/file.txt"));
    }

    #[test]
    fn error_context_for_multiple_write_lists_all_targets() {
        let writes = vec![
            WriteOp {
                target: PathBuf::from("/output/file1.txt"),
                content: "content1".to_string(),
                target_exists: false,
            },
            WriteOp {
                target: PathBuf::from("/output/file2.txt"),
                content: "content2".to_string(),
                target_exists: false,
            },
        ];
        let op = TemplateOperation::MultipleWrite { writes };
        let context = op.error_context();
        assert!(context.contains("write multiple files"));
        assert!(context.contains("/output/file1.txt"));
        assert!(context.contains("/output/file2.txt"));
    }
}
