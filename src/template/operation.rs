use std::path::PathBuf;

#[derive(Debug)]
pub enum TemplateOperation {
    Copy { source: PathBuf, target: PathBuf, target_exists: bool },
    Write { target: PathBuf, content: String, target_exists: bool },
    CreateDirectory { target: PathBuf, target_exists: bool },
    Ignore { source: PathBuf },
}

impl TemplateOperation {
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

            TemplateOperation::Write { target, content: _, target_exists } => {
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
        let expected = format!(
            "Copying '{}' to '{}' (overwriting existing file)",
            &source.display(),
            &target.display()
        );

        let copy = TemplateOperation::Copy { source, target, target_exists: true };
        let actual = copy.get_message(true);
        assert_eq!(actual, expected);
    }
}
