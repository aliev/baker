use globset::GlobSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::ioutils::path_to_str;
use crate::renderer::TemplateRenderer;

use super::operation::TemplateOperation;

pub struct TemplateProcessor<'a, P: AsRef<Path>> {
    /// Dependencies
    engine: &'a dyn TemplateRenderer,
    bakerignore: &'a GlobSet,

    /// Other
    template_root: P,
    output_root: P,
    answers: &'a serde_json::Value,
}

impl<'a, P: AsRef<Path>> TemplateProcessor<'a, P> {
    pub fn new(
        engine: &'a dyn TemplateRenderer,
        template_root: P,
        output_root: P,
        answers: &'a serde_json::Value,
        bakerignore: &'a GlobSet,
    ) -> Self {
        Self { engine, template_root, output_root, answers, bakerignore }
    }

    /// Validates whether the `rendered_entry` is properly rendered by comparing its components
    /// with those of the original `template_entry`. The validation ensures no parts of the path
    /// are empty after rendering.
    ///
    /// Example:
    /// Given the following `template_entry`:
    /// `template_root/{% if create_tests %}tests{% endif %}/`
    /// And a corresponding `rendered_entry`:
    /// `template_root/tests/`
    //
    /// The `has_valid_rendered_path_parts` function splits both paths by "/" and compares
    /// their parts. If none of the parts are empty, the function concludes that the path
    /// was correctly rendered and proceeds with processing.
    ///
    /// However, if the `create_tests` value in `self.answers` is `false`, the rendered path
    /// will look like this:
    /// `template_root//`
    ///
    /// When compared with the original `template_entry`, `template_root/{% if create_tests %}tests{% endif %}/`,
    /// the function detects that one of the parts is empty (due to the double "//").
    /// In such cases, it considers the rendered path invalid and skips further processing.
    ///
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

    /// Checks if the provided path is a Baker template file (with .baker.j2 extension)
    ///
    /// This function analyzes the filename and its extensions to determine if the file
    /// is a valid Baker template. A valid template must follow the `*.baker.j2` format.
    ///
    /// # Arguments
    ///
    /// * `path` - A path to the file that implements AsRef<Path> trait
    ///
    /// # Returns
    ///
    /// * `true` - if the file has .baker.j2 extension
    /// * `false` - in the following cases:
    ///   - path doesn't contain a filename
    ///   - filename cannot be converted to a string
    ///   - file has fewer than two extensions
    ///   - extensions don't match .baker.j2 format
    ///
    fn is_template_file<T: AsRef<Path>>(&self, path: T) -> bool {
        let path = path.as_ref();
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => return false,
        };

        let parts: Vec<&str> = file_name.split('.').collect();

        if parts.len() < 2 {
            return false;
        }

        let prev = parts[parts.len() - 2];

        prev == "baker" && parts.last() == Some(&"j2")
    }

    /// The `template_entry` file or directory name may contain a template string.
    /// This allows the system to determine whether to create a file or directory and
    /// how to resolve its name based on provided template data.
    ///
    /// For example, if the file or directory name contains the string:
    /// `{{filename}}.txt`, it will be rendered as `my_file_name.txt`
    /// if the value for "filename" in `self.answers` is "my_file_name".
    ///
    /// Additionally, conditions can be applied. For instance, if the file or directory name
    /// has an empty value, it will not be created.
    /// Example: `{% if create_tests %}tests{% endif %}/` will create the directory only
    /// if `create_tests` in `self.answers` evaluates to true.
    ///
    fn render_template_entry(&self, template_entry: &PathBuf) -> Result<PathBuf> {
        let rendered_entry = self.engine.render_path(template_entry, self.answers)?;
        let rendered_entry = rendered_entry.as_str();

        if !self
            .has_valid_rendered_path_parts(path_to_str(&template_entry)?, rendered_entry)
        {
            return Err(Error::ProcessError {
                source_path: rendered_entry.to_string(),
                e: "The rendered path is not valid".to_string(),
            });
        }

        // Removes the `.baker.j2` suffix to create the target filename with its actual extension.
        //
        // The following lines check whether the `template_entry` is a template file by
        // determining if its filename ends with a double extension that includes `.baker.j2`.
        // For example:
        // - `README.md.baker.j2` will be considered a template file because it has the double
        //   extensions `.baker` and `.j2`.
        // - `.dockerignore.baker.j2` will also be considered a template file since it includes
        //   `.baker` and `.j2` as extensions.
        //
        // However, filenames like `template.j2` or `README.md` will not be considered
        // template files because they lack a double extension with `.j2`.
        //
        let result = if self.is_template_file(template_entry) {
            rendered_entry.strip_suffix(".baker.j2").unwrap_or(rendered_entry)
        } else {
            rendered_entry
        };

        // Converts the `rendered_entry` slice to a `PathBuf` for easier manipulation
        // in subsequent operations.
        Ok(PathBuf::from(result))
    }

    /// Constructs the `target_path` from `rendered_entry`, which represents the
    /// actual path to the file or directory that will be created in `output_root`.
    //
    /// The `target_path` is built by replacing the `template_root` prefix with the `output_root` prefix.
    /// Example:
    /// If `rendered_entry` is:
    /// `PathBuf("template_root/tests/__init__.py")`
    ///
    /// The `template_root` prefix is replaced with `output_root`, resulting in:
    /// `PathBuf("output_root/tests/__init__.py")`
    ///
    /// Here, `output_root` is the directory where the rendered file or directory will be saved.
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
        match (template_entry.is_file(), self.is_template_file(&template_entry)) {
            // Template file
            (true, true) => {
                let template_content = fs::read_to_string(&template_entry)?;
                let content = self.engine.render(&template_content, self.answers)?;

                Ok(TemplateOperation::Write {
                    target: target_path,
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
        ignore::parse_bakerignore_file, renderer::MiniJinjaRenderer,
        template::operation::TemplateOperation,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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
    #[ignore = r#"because:

        The template structure
            template_root/
                {{file_name}}
        Expected output
            output_root/
                hello_world.txt
        Answers are:
            {"file_name": "hello_world.txt.baker.j2", "greetings": "Hello, World"}
        Actual result is:
            Copy {
                source: "/template_root/{{file_name}}",
                target: "/output_root/hello_world.txt.baker.j2",
                target_exists: false,
            }
    "#]
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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

        let engine = Box::new(MiniJinjaRenderer::new());
        let ignored_patterns = parse_bakerignore_file(template_root).unwrap();
        let processor = TemplateProcessor::new(
            engine.as_ref(),
            &template_root,
            &output_root,
            &answers,
            &ignored_patterns,
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
