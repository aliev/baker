use std::path::PathBuf;

#[derive(Debug)]
pub enum TemplateOperation {
    Copy { source: PathBuf, target: PathBuf, target_exists: bool },
    Write { target: PathBuf, content: String, target_exists: bool },
    CreateDirectory { target: PathBuf, target_exists: bool },
    Ignore { source: PathBuf },
}

impl TemplateOperation {
    /// Gets a message describing the operation and its status.
    ///
    /// # Arguments
    /// * `user_confirmed_overwrite` - Whether the user has confirmed overwriting existing files
    ///
    /// # Returns
    /// * `String` - A descriptive message about the operation
    pub fn get_message(&self, user_confirmed_overwrite: bool) -> String {
        match self {
            TemplateOperation::Copy { source, target, target_exists } => {
                if *target_exists {
                    if user_confirmed_overwrite {
                        format!(
                            "Copying '{}' to '{}' (overwriting existing file)",
                            source.display(),
                            target.display()
                        )
                    } else {
                        format!(
                            "Skipping copy of '{}' to '{}' (target already exists)",
                            source.display(),
                            target.display()
                        )
                    }
                } else {
                    format!("Copying '{}' to '{}'", source.display(), target.display())
                }
            }

            TemplateOperation::CreateDirectory { target, target_exists } => {
                if *target_exists {
                    format!(
                        "Skipping directory creation '{}' (already exists)",
                        target.display()
                    )
                } else {
                    format!("Creating directory '{}'", target.display())
                }
            }

            TemplateOperation::Write { target, target_exists, .. } => {
                if *target_exists {
                    if user_confirmed_overwrite {
                        format!(
                            "Writing to '{}' (overwriting existing file)",
                            target.display()
                        )
                    } else {
                        format!(
                            "Skipping write to '{}' (target already exists)",
                            target.display()
                        )
                    }
                } else {
                    format!("Writing to '{}'", target.display())
                }
            }

            TemplateOperation::Ignore { source } => {
                format!("Ignoring '{}' (matches ignore pattern)", source.display())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works_1() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = true;
        let expected = format!(
            "Copying '{}' to '{}' (overwriting existing file)",
            &source.display(),
            &target.display()
        );

        let copy = TemplateOperation::Copy { source, target, target_exists: true };
        let actual = copy.get_message(user_confirmed_overwrite);
        assert_eq!(actual, expected);
    }

    #[test]
    fn it_works_2() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected = format!(
            "Skipping copy of '{}' to '{}' (target already exists)",
            &source.display(),
            &target.display()
        );

        let copy = TemplateOperation::Copy { source, target, target_exists: true };
        let actual = copy.get_message(user_confirmed_overwrite);
        assert_eq!(actual, expected);
    }

    #[test]
    fn it_works_3() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected =
            format!("Copying '{}' to '{}'", &source.display(), &target.display());

        let copy = TemplateOperation::Copy { source, target, target_exists: false };
        let actual = copy.get_message(user_confirmed_overwrite);
        assert_eq!(actual, expected);
    }
    #[test]
    fn it_works_4() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected = format!(
            "Skipping directory creation '{}' (already exists)",
            &target.display()
        );

        let copy = TemplateOperation::CreateDirectory { target, target_exists: true };
        let actual = copy.get_message(user_confirmed_overwrite);
        assert_eq!(actual, expected);
    }
    #[test]
    fn it_works_5() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected = format!("Creating directory '{}'", &target.display());

        let copy = TemplateOperation::CreateDirectory { target, target_exists: false };
        let actual = copy.get_message(user_confirmed_overwrite);
        assert_eq!(actual, expected);
    }
    #[test]
    fn it_works_6() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = true;
        let expected =
            format!("Writing to '{}' (overwriting existing file)", &target.display());

        let copy = TemplateOperation::Write {
            target,
            target_exists: true,
            content: "".to_string(),
        };
        let actual = copy.get_message(user_confirmed_overwrite);
        assert_eq!(actual, expected);
    }
    #[test]
    fn it_works_7() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected =
            format!("Skipping write to '{}' (target already exists)", &target.display());

        let copy = TemplateOperation::Write {
            target,
            target_exists: true,
            content: "".to_string(),
        };
        let actual = copy.get_message(user_confirmed_overwrite);
        assert_eq!(actual, expected);
    }
    #[test]
    fn it_works_8() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected = format!("Writing to '{}'", &target.display());

        let copy = TemplateOperation::Write {
            target,
            target_exists: false,
            content: "".to_string(),
        };
        let actual = copy.get_message(user_confirmed_overwrite);
        assert_eq!(actual, expected);
    }
    #[test]
    fn it_works_9() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected =
            format!("Ignoring '{}' (matches ignore pattern)", &source.display());

        let copy = TemplateOperation::Ignore { source };
        let actual = copy.get_message(user_confirmed_overwrite);
        assert_eq!(actual, expected);
    }
}
