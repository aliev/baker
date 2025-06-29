use crate::error::{Error, Result};
use crate::ioutils::path_to_str;
use crate::renderer::TemplateRenderer;
use globset::GlobSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::operation::TemplateOperation;

pub struct TemplateProcessor<'a, P: AsRef<Path>> {
    /// Dependencies
    engine: &'a dyn TemplateRenderer,
    bakerignore: &'a GlobSet,

    /// Other
    template_root: P,
    output_root: P,
    answers: &'a serde_json::Value,
    template_suffix: &'a str,
}

impl<'a, P: AsRef<Path>> TemplateProcessor<'a, P> {
    pub fn new(
        engine: &'a dyn TemplateRenderer,
        template_root: P,
        output_root: P,
        answers: &'a serde_json::Value,
        bakerignore: &'a GlobSet,
        template_suffix: &'a str,
    ) -> Self {
        Self { engine, template_root, output_root, answers, bakerignore, template_suffix }
    }

    /// Validates whether the `rendered_entry` is properly rendered by comparing its components
    /// with those of the original `template_entry`. The validation ensures no parts of the path
    /// are empty after rendering.
    ///
    /// # Arguments
    /// * `template_path` - The original template path
    /// * `rendered_path` - The path after rendering with template variables
    ///
    /// # Returns
    /// * `bool` - Whether the rendered path is valid
    ///
    /// # Examples
    ///
    /// Valid case:
    /// - Template path: `template_root/{% if create_tests %}tests{% endif %}/`
    /// - Rendered path (when create_tests=true): `template_root/tests/`
    ///
    /// Invalid case:
    /// - Template path: `template_root/{% if create_tests %}tests{% endif %}/`
    /// - Rendered path (when create_tests=false): `template_root//` (contains empty part)
    ///
    fn has_valid_rendered_path_parts<S: AsRef<str>>(
        &self,
        template_path: S,
        rendered_path: S,
    ) -> bool {
        let template_path = template_path.as_ref();
        let rendered_path = rendered_path.as_ref();
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

    /// Checks if the provided path is a Baker template file by checking if the file's extension
    /// is the same as `template_suffix` (defaults to .baker.j2)
    ///
    /// # Arguments
    /// * `path` - A path to the file
    ///
    /// # Returns
    /// * `true` - if the file has the same extension as the `template_suffix`
    /// * `false` - if the path is not a template file
    ///
    fn is_template_file<T: AsRef<Path>>(&self, path: T) -> bool {
        let path = path.as_ref();

        path.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|file_name| file_name.ends_with(self.template_suffix))
    }

    /// Renders a template entry path with template variables.
    ///
    /// # Arguments
    /// * `template_entry` - The template path to render
    ///
    /// # Returns
    /// * `Result<PathBuf>` - The rendered path or an error
    ///
    fn render_template_entry(&self, template_entry: &PathBuf) -> Result<PathBuf> {
        let rendered_entry = self.engine.render_path(template_entry, self.answers)?;

        if !self
            .has_valid_rendered_path_parts(path_to_str(template_entry)?, &rendered_entry)
        {
            return Err(Error::ProcessError {
                source_path: rendered_entry.to_string(),
                e: "The rendered path is not valid".to_string(),
            });
        }

        Ok(PathBuf::from(rendered_entry))
    }

    /// Removes the designated template suffix (by default it's `.baker.j2`) from a template file path.
    ///
    /// # Arguments
    /// * `target_path` - Path with possible template suffix
    ///
    /// # Returns
    /// * `Result<PathBuf>` - Path with suffix removed
    ///
    fn remove_template_suffix(&self, target_path: &PathBuf) -> Result<PathBuf> {
        let target_path_str = path_to_str(target_path)?;
        let target =
            target_path_str.strip_suffix(self.template_suffix).unwrap_or(target_path_str);

        Ok(PathBuf::from(target))
    }

    /// Constructs the target path for a rendered entry.
    ///
    /// # Arguments
    /// * `rendered_entry` - The rendered entry path
    /// * `template_entry` - The original template entry path
    ///
    /// # Returns
    /// * `Result<PathBuf>` - The target path in the output directory
    ///
    fn get_target_path(
        &self,
        rendered_entry: &Path,
        template_entry: &Path,
    ) -> Result<PathBuf> {
        let target_path = rendered_entry
            .strip_prefix(self.template_root.as_ref())
            .map_err(|e| Error::ProcessError {
                source_path: template_entry.display().to_string(),
                e: e.to_string(),
            })?;
        Ok(self.output_root.as_ref().join(target_path))
    }

    /// Processes a template entry and determines the appropriate operation.
    ///
    /// # Arguments
    /// * `template_entry` - The template entry to process
    ///
    /// # Returns
    /// * `Result<TemplateOperation>` - The operation to perform
    ///
    pub fn process(&self, template_entry: P) -> Result<TemplateOperation> {
        let template_entry = template_entry.as_ref().to_path_buf();
        let rendered_entry = self.render_template_entry(&template_entry)?;
        let target_path = self.get_target_path(&rendered_entry, &template_entry)?;
        let target_exists = target_path.exists();

        // Skip if entry is in .bakerignore
        if self.bakerignore.is_match(&template_entry) {
            return Ok(TemplateOperation::Ignore { source: rendered_entry });
        }

        // Handle different types of entries
        match (template_entry.is_file(), self.is_template_file(&rendered_entry)) {
            // Template file
            (true, true) => {
                let template_content = fs::read_to_string(&template_entry)?;
                let template_name =
                    template_entry.file_name().and_then(|name| name.to_str());
                let content =
                    self.engine.render(&template_content, self.answers, template_name)?;

                Ok(TemplateOperation::Write {
                    target: self.remove_template_suffix(&target_path)?,
                    content,
                    target_exists,
                })
            }
            // Regular file
            (true, false) => Ok(TemplateOperation::Copy {
                source: template_entry,
                target: target_path,
                target_exists,
            }),
            // Directory
            _ => Ok(TemplateOperation::CreateDirectory {
                target: target_path,
                target_exists,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use fs::File;
    use serde_json::json;
    use tempfile::TempDir;

    use crate::{
        ignore::parse_bakerignore_file,
        template::{get_template_engine, operation::TemplateOperation},
    };

    use super::*;

    /// The template structure
    /// template_root/
    ///   {{file_name}}.txt.baker.j2
    ///
    /// Expected output
    /// output_root/
    ///   hello_world.txt
    ///
    /// Because answers are
    /// {"file_name": "hello_world", "greetings": "Hello, World"}
    ///
    #[test]
    fn it_works_1() {
        let answers = json!({"file_name": "hello_world", "greetings": "Hello, World"});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let file_path = template_root.join("{{file_name}}.txt.baker.j2");

        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{greetings}}").unwrap();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&file_path.as_path()).unwrap();

        match result {
            TemplateOperation::Write { target, content, target_exists } => {
                assert_eq!(target, output_root.join("hello_world.txt"));
                assert_eq!(content, "Hello, World");
                assert!(!target_exists);
            }
            _ => panic!("Expected Write operation"),
        }
    }

    /// The template structure
    /// template_root/
    ///   hello_world.txt
    ///
    /// Expected output
    /// output_root/
    ///   hello_world.txt
    ///
    /// Because answers are
    /// {}
    ///
    #[test]
    fn it_works_3() {
        let answers = json!({});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let file_path = template_root.join("hello_world.txt");

        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"Hello, World").unwrap();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&file_path.as_path()).unwrap();

        match result {
            TemplateOperation::Copy { source, target, target_exists } => {
                assert_eq!(target, output_root.join("hello_world.txt"));
                assert_eq!(source, template_root.join("hello_world.txt"));
                assert!(!target_exists);
            }
            _ => panic!("Expected Copy operation"),
        }
    }

    /// The template structure
    /// template_root/
    ///   {{directory_name}}/file_name.txt
    ///
    /// Expected output
    /// output_root/
    ///   hello/world.txt
    ///
    /// Because answers are
    /// {"directory_name": "hello"}
    ///
    #[test]
    fn it_works_4() {
        let answers = json!({"directory_name": "hello", "greetings": "Hello, World"});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let nested_directory_path = template_root.join("{{directory_name}}");

        std::fs::create_dir_all(&nested_directory_path).unwrap();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let file_path = nested_directory_path.join("file_name.txt.baker.j2");

        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{greetings}}").unwrap();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&file_path.as_path()).unwrap();

        match result {
            TemplateOperation::Write { content, target, target_exists } => {
                assert_eq!(content, "Hello, World");
                assert_eq!(target, output_root.join("hello").join("file_name.txt"));
                assert!(!target_exists);
            }
            _ => panic!("Expected Write operation"),
        }
    }

    /// The template structure
    /// template_root/
    ///   {{directory_name}}/{{file_name}}.txt
    ///
    /// Expected output
    /// output_root/
    ///
    /// Because answers are
    /// {"file_name": "world"}
    ///
    #[test]
    fn it_works_5() {
        let answers = json!({"file_name": "world.txt"});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let nested_directory_path = template_root.join("{{directory_name}}");

        std::fs::create_dir_all(&nested_directory_path).unwrap();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let file_path = nested_directory_path.join("{{file_name}}.txt");

        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{greetings}}").unwrap();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&file_path.as_path());
        match result {
            Err(Error::ProcessError { e, .. }) => {
                assert_eq!(e, "The rendered path is not valid");
            }
            _ => panic!("Expected ProcessError"),
        }
    }

    /// The template structure
    /// template_root/
    ///   {% if create_dir %}hello{% endif %}/
    ///
    /// Expected output
    /// output_root/
    ///   hello/
    ///
    /// Because answers are
    /// {"create_dir": true}
    ///
    #[test]
    fn it_works_6() {
        let answers = json!({"create_dir": true});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let nested_directory_path =
            template_root.join("{% if create_dir %}hello{% endif %}");

        std::fs::create_dir_all(&nested_directory_path).unwrap();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&nested_directory_path.as_path()).unwrap();
        match result {
            TemplateOperation::CreateDirectory { target, target_exists } => {
                assert_eq!(target, output_root.join("hello"));
                assert!(!target_exists);
            }
            _ => panic!("Expected CreateDirectory operation"),
        }
    }

    /// The template structure
    /// template_root/
    ///   {% if create_dir %}hello{% endif %}/
    ///
    /// Expected output
    /// output_root/
    ///
    /// Because answers are
    /// {"create_dir": false}
    ///
    #[test]
    fn it_works_7() {
        let answers = json!({"create_dir": false});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let nested_directory_path =
            template_root.join("{% if create_dir %}hello{% endif %}");

        std::fs::create_dir_all(&nested_directory_path).unwrap();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&nested_directory_path.as_path());
        match result {
            Err(Error::ProcessError { e, .. }) => {
                assert_eq!(e, "The rendered path is not valid");
            }
            _ => panic!("Expected ProcessError"),
        }
    }

    /// The template structure
    /// template_root/
    ///   {% if create_dir %}hello{% endif %}/
    ///     file_name.txt
    ///
    /// Expected output
    /// output_root/
    ///   hello/
    ///     file_name.txt
    ///
    /// Because answers are
    /// {"create_dir": true}
    ///
    #[test]
    fn it_works_8() {
        let answers = json!({"create_dir": true});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let nested_directory_path =
            template_root.join("{% if create_dir %}hello{% endif %}");

        std::fs::create_dir_all(&nested_directory_path).unwrap();

        let file_path = nested_directory_path.join("file_name.txt");

        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{greetings}}").unwrap();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&file_path.as_path()).unwrap();
        match result {
            TemplateOperation::Copy { source, target, target_exists } => {
                assert_eq!(target, output_root.join("hello").join("file_name.txt"));
                assert_eq!(source, file_path);
                assert!(!target_exists);
            }
            _ => panic!("Expected Copy operation"),
        }
    }

    /// The template structure
    /// template_root/
    ///   {{file_name}}
    ///
    /// Expected output
    /// output_root/
    ///   hello_world.txt
    ///
    /// Because answers are
    /// {"file_name": "hello_world.txt.baker.j2", "greetings": "Hello, World"}
    ///
    #[test]
    fn it_works_9() {
        let answers =
            json!({"file_name": "hello_world.txt.baker.j2", "greetings": "Hello, World"});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let file_path = template_root.join("{{file_name}}");

        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{greetings}}").unwrap();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&file_path.as_path()).unwrap();

        match result {
            TemplateOperation::Write { target, content, target_exists } => {
                assert_eq!(target, output_root.join("hello_world.txt"));
                assert_eq!(content, "Hello, World");
                assert!(!target_exists);
            }
            _ => panic!("Expected Write operation"),
        }
    }

    /// The template structure
    /// template_root/
    ///   hello_world.j2
    ///
    /// Expected output
    /// output_root/
    ///   hello_world.j2
    ///
    /// Because answers are
    /// {"greetings": "Hello, World"}
    ///
    #[test]
    fn it_works_10() {
        let answers = json!({"greetings": "Hello, World"});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let file_path = template_root.join("hello_world.j2");

        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{greetings}}").unwrap();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&file_path.as_path()).unwrap();

        match result {
            TemplateOperation::Copy { target, source, target_exists } => {
                assert_eq!(source, template_root.join("hello_world.j2"));
                assert_eq!(target, output_root.join("hello_world.j2"));
                assert!(!target_exists);
            }
            _ => panic!("Expected Copy operation"),
        }
    }

    /// The template structure
    /// template_root/
    ///   README.baker.j2
    ///
    /// Expected output
    /// output_root/
    ///   README
    ///
    /// Because answers are
    /// {"first_name": "Ali", "last_name": "Aliyev"}
    ///
    #[test]
    fn it_works_11() {
        let answers = json!({"first_name": "Ali", "last_name": "Aliyev"});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let file_path = template_root.join("README.baker.j2");

        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{first_name}} {{last_name}}").unwrap();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&file_path.as_path()).unwrap();

        match result {
            TemplateOperation::Write { target, target_exists, content } => {
                assert_eq!(target, output_root.join("README"));
                assert_eq!(content, "Ali Aliyev");
                assert!(!target_exists);
            }
            _ => panic!("Expected Copy operation"),
        }
    }
    /// The template structure
    /// template_root/
    ///   {{file_name}}.baker.j2
    ///
    /// Expected output
    /// output_root/
    ///   README
    ///
    /// Because answers are
    /// {"first_name": "Ali", "last_name": "Aliyev", "file_name": "README"}
    ///
    #[test]
    fn it_works_12() {
        let answers =
            json!({"first_name": "Ali", "last_name": "Aliyev", "file_name": "README"});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let file_path = template_root.join("{{file_name}}.baker.j2");

        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{first_name}} {{last_name}}").unwrap();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&file_path.as_path()).unwrap();

        match result {
            TemplateOperation::Write { target, target_exists, content } => {
                assert_eq!(target, output_root.join("README"));
                assert_eq!(content, "Ali Aliyev");
                assert!(!target_exists);
            }
            _ => panic!("Expected Copy operation"),
        }
    }
    /// The template structure
    /// template_root/
    ///   {{file_name}}.baker.j2
    ///
    /// Expected output
    /// output_root/
    ///
    /// Because answers are
    /// {}
    ///
    #[test]
    #[ignore = r#"because:

        The template structure
            template_root/
                {{file_name}}.baker.j2
        Expected output
            output_root/
        Answers are:
            {}
        Actual result is:
            Write {
                content: " ",
                target: "/output_root/",
                target_exists: false,
            }
        Expected result: `Error::ProcessError`
    "#]
    fn it_works_14() {
        let answers = json!({});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let file_path = template_root.join("{{file_name}}.baker.j2");

        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{first_name}} {{last_name}}").unwrap();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&file_path.as_path());
        match result {
            Err(Error::ProcessError { e, .. }) => {
                assert_eq!(e, "The rendered path is not valid");
            }
            _ => panic!("Expected ProcessError"),
        }
    }
    /// The template structure
    /// template_root/
    ///   {{file_name}}
    ///
    /// Expected output
    /// output_root/
    ///
    /// Because answers are
    /// {}
    ///
    #[test]
    fn it_works_15() {
        let answers = json!({});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let file_path = template_root.join("{{file_name}}");

        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{first_name}} {{last_name}}").unwrap();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&file_path.as_path());

        match result {
            Err(Error::ProcessError { e, .. }) => {
                assert_eq!(e, "The rendered path is not valid");
            }
            _ => panic!("Expected ProcessError"),
        }
    }
    /// The template structure
    /// template_root/
    ///   {{file_name}}.txt
    ///
    /// Expected output
    /// output_root/
    ///
    /// Because answers are
    /// {}
    ///
    #[test]
    #[ignore = r#"because:

        The template structure
            template_root/
                {{file_name}}.txt
        Expected output
            output_root/
        Answers are:
            {}
        Actual result is:
            Copy {
                source: "/template_root/{{file_name}}.txt",
                target: "/output_root/.txt",
                target_exists: false,
            }
        Expected result: `Error::ProcessError`
    "#]
    fn it_works_16() {
        let answers = json!({});
        let template_root = TempDir::new().unwrap();
        let template_root = template_root.path();

        let output_root = TempDir::new().unwrap();
        let output_root = output_root.path();

        let file_path = template_root.join("{{file_name}}.txt");

        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{first_name}} {{last_name}}").unwrap();

        let engine = get_template_engine();
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            &engine,
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
            ".baker.j2",
        );

        let result = processor.process(&file_path.as_path());
        match result {
            Err(Error::ProcessError { e, .. }) => {
                assert_eq!(e, "The rendered path is not valid");
            }
            _ => panic!("Expected ProcessError"),
        }
    }
}
