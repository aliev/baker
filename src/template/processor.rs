use crate::{
    cli::context::GenerationContext,
    error::{Error, Result},
    ext::PathExt,
    renderer::TemplateRenderer,
    template::operation::{TemplateOperation, TemplateOperation::MultipleWrite, WriteOp},
};
use globset::GlobSet;
use log::debug;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

pub struct TemplateProcessor<'a, P: AsRef<Path>> {
    /// Dependencies
    engine: &'a dyn TemplateRenderer,
    bakerignore: &'a GlobSet,

    /// Other
    template_root: P,
    output_root: P,
    answers: &'a serde_json::Value,
    template_config: TemplateConfig<'a>,
    loop_detector: Regex,
    loop_end_regex: Regex,
}

pub struct TemplateConfig<'a> {
    pub template_suffix: &'a str,
    pub loop_separator: &'a str,
    pub loop_content_separator: &'a str,
}

impl<'a> TemplateProcessor<'a, PathBuf> {
    pub fn new(
        engine: &'a dyn TemplateRenderer,
        context: &'a GenerationContext,
        bakerignore: &'a GlobSet,
    ) -> Self {
        let config = context.config();
        let template_config = TemplateConfig {
            template_suffix: config.template_suffix.as_str(),
            loop_separator: config.loop_separator.as_str(),
            loop_content_separator: config.loop_content_separator.as_str(),
        };

        Self {
            engine,
            bakerignore,
            template_root: context.template_root().clone(),
            output_root: context.output_root().clone(),
            answers: context.answers(),
            template_config,
            loop_detector: Regex::new(r"\{\%\s*for\s+.*in.*\%\}")
                .expect("valid for-loop regex"),
            loop_end_regex: Regex::new(r"(\{\%\s*endfor\s*\%\})")
                .expect("valid endfor regex"),
        }
    }
}

impl<'a, P: AsRef<Path>> TemplateProcessor<'a, P> {
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
    /// Invalid case (placeholder at start of filename):
    /// - Template path: `template_root/{{file_name}}.txt`
    /// - Rendered path (when file_name is undefined): `template_root/.txt`
    ///
    fn rendered_path_has_valid_parts<S: AsRef<str>>(
        &self,
        template_path: S,
        rendered_path: S,
    ) -> bool {
        let template_path = template_path.as_ref();
        let rendered_path = rendered_path.as_ref();
        let template_path_parts: Vec<&str> =
            template_path.split(std::path::MAIN_SEPARATOR).collect();
        let rendered_path_parts: Vec<&str> =
            rendered_path.split(std::path::MAIN_SEPARATOR).collect();

        for (template_part, rendered_part) in
            template_path_parts.iter().zip(rendered_path_parts.iter())
        {
            // Check if a non-empty template part became empty
            if !template_part.is_empty() && rendered_part.is_empty() {
                return false;
            }

            // Check if a template placeholder at the start of a filename rendered to empty
            // This catches cases like "{{file_name}}.txt" -> ".txt" or "{{file_name}}.baker.j2" -> ".baker.j2"
            if template_part.starts_with("{{")
                && !template_part.starts_with("{{.")
                && rendered_part.starts_with('.')
            {
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

        path.file_name().and_then(|n| n.to_str()).is_some_and(|file_name| {
            file_name.ends_with(self.template_config.template_suffix)
        })
    }

    /// Returns the relative path from template root for use in error messages.
    ///
    /// # Arguments
    /// * `path` - The full path to compute relative path for
    ///
    /// # Returns
    /// * `Option<String>` - The relative path as a string, or None if it cannot be computed
    ///
    fn get_template_name(&self, path: &Path) -> Option<String> {
        path.strip_prefix(self.template_root.as_ref())
            .ok()
            .and_then(|p| p.to_str())
            .map(|s| s.replace('\\', "/"))
    }

    /// Renders a template entry path with template variables.
    ///
    /// # Arguments
    /// * `template_entry` - The template path to render
    ///
    /// # Returns
    /// * `Result<PathBuf>` - The rendered path or an error
    ///
    fn render_template_entry(&self, template_entry: &Path) -> Result<PathBuf> {
        let rendered_entry = self.engine.render_path(template_entry, self.answers)?;

        if !self.rendered_path_has_valid_parts(
            template_entry.to_str_checked()?,
            &rendered_entry,
        ) {
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
    fn remove_template_suffix(&self, target_path: &Path) -> Result<PathBuf> {
        let target_path_str = target_path.to_str_checked()?;
        let target = target_path_str
            .strip_suffix(self.template_config.template_suffix)
            .unwrap_or(target_path_str);

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

    /// Returns true if the path contains any MiniJinja for-loop block delimiters anywhere in the path (not just filename).
    pub fn is_template_with_loop<T: AsRef<Path>>(&self, template_entry: T) -> bool {
        let path_str = template_entry.as_ref().to_string_lossy();
        self.loop_detector.is_match(&path_str)
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
                let relative_path = self.get_template_name(&template_entry);
                if self.is_template_with_loop(&template_entry) {
                    debug!("Processing loop template file: {}", template_entry.display());
                    return self
                        .render_loop_template_file(&template_entry, template_name);
                }
                let content = self
                    .engine
                    .render(&template_content, self.answers, template_name)
                    .map_err(|e| Error::ProcessError {
                        source_path: relative_path
                            .unwrap_or_else(|| template_entry.display().to_string()),
                        e: e.to_string(),
                    })?;

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

    /// Renders the loop template file by injecting content into the loop and rendering the result.
    ///
    /// # Arguments
    /// * `template_entry` - The template path to render
    /// * `template_name` - The name of the template file
    ///
    /// # Returns
    /// * `Result<String>` - The rendered content with injected values
    ///
    fn render_loop_template_file(
        &self,
        template_entry: &Path,
        template_name: Option<&str>,
    ) -> Result<TemplateOperation> {
        let relative_path = self.get_template_name(template_entry);
        let template_parent_dir =
            template_entry.parent().map(PathBuf::from).ok_or_else(|| {
                Error::ProcessError {
                    source_path: template_entry.display().to_string(),
                    e: "Failed to extract parent directory from template_entry"
                        .to_string(),
                }
            })?;
        let rendered_parent_dir =
            self.render_template_entry(template_parent_dir.as_path())?;
        debug!(
            "Rendered parent directory for loop template: {}",
            rendered_parent_dir.display()
        );
        let raw_template_content = fs::read_to_string(template_entry)?;
        debug!("Raw loop template content: {raw_template_content}");
        let template_with_injected_content =
            self.inject_loop_content(template_entry, &raw_template_content)?;
        debug!("Loop template after content injection: {template_with_injected_content}");
        let rendered_content = self
            .engine
            .render(&template_with_injected_content, self.answers, template_name)
            .map_err(|e| Error::ProcessError {
                source_path: relative_path
                    .unwrap_or_else(|| template_entry.display().to_string()),
                e: e.to_string(),
            })?;
        debug!("Rendered loop template content: {rendered_content}");
        let write_operations = self.collect_loop_write_ops(
            &rendered_content,
            rendered_parent_dir.as_path(),
            template_entry,
        )?;
        Ok(MultipleWrite { writes: write_operations })
    }

    fn split_content(&self, input: &str) -> Vec<(String, String)> {
        input
            .split(self.template_config.loop_separator)
            .filter_map(|part| {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    return None;
                }
                let mut sections =
                    trimmed.splitn(2, self.template_config.loop_content_separator);
                let filename = sections.next()?.trim().to_string();
                let content = sections.next()?.trim().to_string();
                Some((filename, content))
            })
            .collect()
    }

    fn inject_loop_content(
        &self,
        template_entry: &Path,
        raw_template_content: &str,
    ) -> Result<String> {
        let injected_content = format!(
            "{}{}{}$1",
            self.template_config.loop_content_separator,
            raw_template_content,
            self.template_config.loop_separator
        );
        let template_str = template_entry.to_str_checked()?;
        Ok(self.loop_end_regex.replace_all(template_str, injected_content).into_owned())
    }

    fn collect_loop_write_ops(
        &self,
        rendered_content: &str,
        rendered_parent_dir: &Path,
        template_entry: &Path,
    ) -> Result<Vec<WriteOp>> {
        self.split_content(rendered_content)
            .into_iter()
            .map(|(rendered_filename, content)| {
                let mut output_file_path = PathBuf::from(&rendered_filename);
                if !output_file_path.starts_with(rendered_parent_dir) {
                    output_file_path = rendered_parent_dir.join(&rendered_filename);
                }
                let final_output_path =
                    self.get_target_path(&output_file_path, template_entry)?;
                let target_exists = final_output_path.exists();
                Ok(WriteOp {
                    target: self.remove_template_suffix(&final_output_path)?,
                    content,
                    target_exists,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cli::context::GenerationContext, config::ConfigV1, renderer::MiniJinjaRenderer,
        template::operation::TemplateOperation,
    };
    use fs::File;
    use globset::GlobSetBuilder;
    use indexmap::IndexMap;
    use serde_json::json;
    use std::io::Write;
    use tempfile::TempDir;

    fn new_test_processor(
        answers: serde_json::Value,
    ) -> (TempDir, TempDir, TemplateProcessor<'static, PathBuf>) {
        let template_root = TempDir::new().unwrap();
        let output_root = TempDir::new().unwrap();
        let engine = Box::new(MiniJinjaRenderer::new());
        let bakerignore = GlobSetBuilder::new().build().unwrap();

        let mut context = GenerationContext::new(
            template_root.path().to_path_buf(),
            output_root.path().to_path_buf(),
            ConfigV1 {
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
                follow_symlinks: false,
            },
            Vec::new(),
            false,
        );
        context.set_answers(answers);

        let context = Box::leak(Box::new(context));
        let processor = TemplateProcessor::new(
            Box::leak(engine),
            context,
            &*Box::leak(Box::new(bakerignore)),
        );
        (template_root, output_root, processor)
    }

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
    fn renders_template_file_with_answers() {
        let answers = json!({"file_name": "hello_world", "greetings": "Hello, World"});
        let (template_root, output_root, processor) = new_test_processor(answers);
        let file_path = template_root.path().join("{{file_name}}.txt.baker.j2");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{greetings}}\n").unwrap();
        let result = processor.process(file_path).unwrap();
        match result {
            TemplateOperation::Write { target, content, target_exists } => {
                assert_eq!(target, output_root.path().join("hello_world.txt"));
                assert_eq!(content.trim(), "Hello, World");
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
    fn copies_plain_file_without_suffix() {
        let answers = json!({});
        let (template_root, output_root, processor) = new_test_processor(answers);
        let file_path = template_root.path().join("hello_world.txt");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"Hello, World").unwrap();
        let result = processor.process(file_path).unwrap();
        match result {
            TemplateOperation::Copy { source, target, target_exists } => {
                assert_eq!(target, output_root.path().join("hello_world.txt"));
                assert_eq!(source, template_root.path().join("hello_world.txt"));
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
    fn renders_nested_directory_and_file() {
        let answers = json!({"directory_name": "hello", "greetings": "Hello, World"});
        let (template_root, output_root, processor) = new_test_processor(answers);
        let nested_directory_path = template_root.path().join("{{directory_name}}");
        std::fs::create_dir_all(&nested_directory_path).unwrap();
        let file_path = nested_directory_path.join("file_name.txt.baker.j2");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{greetings}}\n").unwrap();
        let result = processor.process(file_path).unwrap();
        match result {
            TemplateOperation::Write { content, target, target_exists } => {
                assert_eq!(content.trim(), "Hello, World");
                assert_eq!(
                    target,
                    output_root.path().join("hello").join("file_name.txt")
                );
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
    fn errors_when_directory_placeholder_missing() {
        let answers = json!({"file_name": "world.txt"});
        let (template_root, _output_root, processor) = new_test_processor(answers);
        let nested_directory_path = template_root.path().join("{{directory_name}}");
        std::fs::create_dir_all(&nested_directory_path).unwrap();
        let file_path = nested_directory_path.join("{{file_name}}.txt");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{greetings}}\n").unwrap();
        let result = processor.process(file_path);
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
    fn creates_directory_when_condition_is_true() {
        let answers = json!({"create_dir": true});
        let (template_root, output_root, processor) = new_test_processor(answers);
        let nested_directory_path =
            template_root.path().join("{% if create_dir %}hello{% endif %}");
        std::fs::create_dir_all(&nested_directory_path).unwrap();
        let result = processor.process(nested_directory_path).unwrap();
        match result {
            TemplateOperation::CreateDirectory { target, target_exists } => {
                assert_eq!(target, output_root.path().join("hello"));
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
    fn errors_when_conditional_directory_missing() {
        let answers = json!({"create_dir": false});
        let (template_root, _output_root, processor) = new_test_processor(answers);
        let nested_directory_path =
            template_root.path().join("{% if create_dir %}hello{% endif %}");
        std::fs::create_dir_all(&nested_directory_path).unwrap();
        let result = processor.process(nested_directory_path);
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
    fn copies_file_inside_conditional_directory() {
        let answers = json!({"create_dir": true});
        let (template_root, output_root, processor) = new_test_processor(answers);
        let nested_directory_path =
            template_root.path().join("{% if create_dir %}hello{% endif %}");
        std::fs::create_dir_all(&nested_directory_path).unwrap();
        let file_path = nested_directory_path.join("file_name.txt");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{greetings}}\n").unwrap();
        let result = processor.process(file_path.clone()).unwrap();
        match result {
            TemplateOperation::Copy { source, target, target_exists } => {
                assert_eq!(
                    target,
                    output_root.path().join("hello").join("file_name.txt")
                );
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
    fn renders_template_filename_and_strips_suffix() {
        let answers =
            json!({"file_name": "hello_world.txt.baker.j2", "greetings": "Hello, World"});
        let (template_root, output_root, processor) = new_test_processor(answers);
        let file_path = template_root.path().join("{{file_name}}");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{greetings}}\n").unwrap();
        let result = processor.process(file_path).unwrap();
        match result {
            TemplateOperation::Write { target, content, target_exists } => {
                assert_eq!(target, output_root.path().join("hello_world.txt"));
                assert_eq!(content.trim(), "Hello, World");
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
    fn copies_non_template_file_with_j2_extension() {
        let answers = json!({"greetings": "Hello, World"});
        let (template_root, output_root, processor) = new_test_processor(answers);
        let file_path = template_root.path().join("hello_world.j2");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{greetings}}\n").unwrap();
        let result = processor.process(file_path).unwrap();
        match result {
            TemplateOperation::Copy { target, source, target_exists } => {
                assert_eq!(source, template_root.path().join("hello_world.j2"));
                assert_eq!(target, output_root.path().join("hello_world.j2"));
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
    fn renders_readme_template_content() {
        let answers = json!({"first_name": "Ali", "last_name": "Aliyev"});
        let (template_root, output_root, processor) = new_test_processor(answers);
        let file_path = template_root.path().join("README.baker.j2");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{first_name}} {{last_name}}\n").unwrap();
        let result = processor.process(file_path).unwrap();
        match result {
            TemplateOperation::Write { target, target_exists, content } => {
                assert_eq!(target, output_root.path().join("README"));
                assert_eq!(content.trim(), "Ali Aliyev");
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
    fn renders_templated_filename_and_content() {
        let answers =
            json!({"first_name": "Ali", "last_name": "Aliyev", "file_name": "README"});
        let (template_root, output_root, processor) = new_test_processor(answers);
        let file_path = template_root.path().join("{{file_name}}.baker.j2");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{first_name}} {{last_name}}\n").unwrap();
        let result = processor.process(file_path).unwrap();
        match result {
            TemplateOperation::Write { target, target_exists, content } => {
                assert_eq!(target, output_root.path().join("README"));
                assert_eq!(content.trim(), "Ali Aliyev");
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
    fn renders_invalid_filename_returns_error() {
        let answers = json!({});
        let (template_root, _output_root, processor) = new_test_processor(answers);
        let file_path = template_root.path().join("{{file_name}}.baker.j2");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{first_name}} {{last_name}}\n").unwrap();
        let result = processor.process(file_path);
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
    fn renders_directory_placeholder_returns_error() {
        let answers = json!({});
        let (template_root, _output_root, processor) = new_test_processor(answers);
        let file_path = template_root.path().join("{{file_name}}");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{first_name}} {{last_name}}\n").unwrap();
        let result = processor.process(file_path);
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
    fn renders_plain_file_placeholder_returns_error() {
        let answers = json!({});
        let (template_root, _output_root, processor) = new_test_processor(answers);
        let file_path = template_root.path().join("{{file_name}}.txt");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{first_name}} {{last_name}}\n").unwrap();
        let result = processor.process(file_path);
        match result {
            Err(Error::ProcessError { e, .. }) => {
                assert_eq!(e, "The rendered path is not valid");
            }
            _ => panic!("Expected ProcessError"),
        }
    }

    #[test]
    fn test_remove_template_suffix() {
        use std::path::Path;
        let (_template_root, _output_root, processor) = new_test_processor(json!({}));
        // Case 1: Path ends with suffix
        let path_with_suffix = Path::new("foo/bar.baker.j2");
        let result = processor.remove_template_suffix(path_with_suffix).unwrap();
        assert_eq!(result, Path::new("foo/bar"));
        // Case 2: Path does not end with suffix
        let path_without_suffix = Path::new("foo/bar.txt");
        let result = processor.remove_template_suffix(path_without_suffix).unwrap();
        assert_eq!(result, Path::new("foo/bar.txt"));
    }

    #[test]
    fn test_get_target_path_strip_prefix_error() {
        use std::path::Path;
        let (_template_root, _output_root, processor) = new_test_processor(json!({}));
        // rendered_entry does not start with template_root, so strip_prefix will fail
        let rendered_entry = Path::new("/not_template_root/file.txt");
        let template_entry = Path::new("/template_root/file.txt");
        let result = processor.get_target_path(rendered_entry, template_entry);
        match result {
            Err(Error::ProcessError { source_path, e }) => {
                assert_eq!(source_path, template_entry.display().to_string());
                assert!(e.contains("prefix"));
            }
            _ => panic!("Expected ProcessError from strip_prefix failure"),
        }
    }

    #[test]
    fn test_process_template_file_write_operation() {
        use crate::template::operation::TemplateOperation;
        use std::io::Write;
        let answers = json!({"name": "test"});
        let (template_root, output_root, processor) = new_test_processor(answers);
        // Create a template file ending with .baker.j2
        let file_path = template_root.path().join("test.txt.baker.j2");
        let mut temp_file = std::fs::File::create(&file_path).unwrap();
        temp_file.write_all(b"{{ name }}").unwrap();
        // Process the template file
        let result = processor.process(file_path).unwrap();
        match result {
            TemplateOperation::Write { target, content, target_exists } => {
                assert_eq!(target, output_root.path().join("test.txt"));
                assert_eq!(content, "test");
                assert!(!target_exists);
            }
            _ => panic!("Expected Write operation for template file"),
        }
    }

    #[test]
    fn test_process_true_true_write_branch() {
        use crate::template::operation::TemplateOperation;
        use std::io::Write;
        let answers = json!({"username": "copilot"});
        let (template_root, output_root, processor) = new_test_processor(answers);
        let file_path = template_root.path().join("user.txt.baker.j2");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{{ username }}").unwrap();
        let result = processor.process(file_path).unwrap();
        match result {
            TemplateOperation::Write { target, content, target_exists } => {
                assert_eq!(target, output_root.path().join("user.txt"));
                assert_eq!(content, "copilot");
                assert!(!target_exists);
            }
            _ => panic!("Expected Write operation for (true, true) match branch"),
        }
    }

    #[test]
    fn test_is_template_with_loop_basic() {
        let (_template_root, _output_root, processor) = new_test_processor(json!({}));
        let path = PathBuf::from(
            "{% for item in items %}{{ item.name }}.txt.baker.j2{% endfor %}",
        );
        assert!(processor.is_template_with_loop(path));
        let path = PathBuf::from("foo.txt.baker.j2");
        assert!(!processor.is_template_with_loop(path));
    }

    #[test]
    fn test_is_template_with_loop_complex() {
        let (_template_root, _output_root, processor) = new_test_processor(json!({}));
        let path = PathBuf::from("{% if msg==hello %}{%for item in items in selectattr(\"name\")%}{{item.name}}.rs.baker.j2{% endfor %}{% endif %}");
        assert!(processor.is_template_with_loop(path));
    }

    #[test]
    fn test_get_template_name_returns_relative_path() {
        let (template_root, _output_root, processor) = new_test_processor(json!({}));
        let nested_path =
            template_root.path().join("subdir").join("nested").join("file.txt");
        let result = processor.get_template_name(&nested_path);
        assert_eq!(result, Some("subdir/nested/file.txt".to_string()));
    }

    #[test]
    fn test_get_template_name_returns_filename_at_root() {
        let (template_root, _output_root, processor) = new_test_processor(json!({}));
        let file_path = template_root.path().join("file.txt");
        let result = processor.get_template_name(&file_path);
        assert_eq!(result, Some("file.txt".to_string()));
    }

    #[test]
    fn test_get_template_name_returns_none_for_unrelated_path() {
        let (_template_root, _output_root, processor) = new_test_processor(json!({}));
        let unrelated_path = PathBuf::from("/some/other/path/file.txt");
        let result = processor.get_template_name(&unrelated_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_template_error_contains_relative_path() {
        let answers = json!({});
        let (template_root, _output_root, processor) = new_test_processor(answers);

        let nested_dir = template_root.path().join("deep").join("nested");
        std::fs::create_dir_all(&nested_dir).unwrap();

        let file_path = nested_dir.join("template.baker.j2");
        let mut temp_file = File::create(&file_path).unwrap();
        temp_file.write_all(b"{% if true %}content without endif").unwrap();

        let result = processor.process(file_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("deep/nested/template.baker.j2")
                || err_msg.contains("deep\\nested\\template.baker.j2"),
            "Error should contain relative path 'deep/nested/template.baker.j2', got: {err_msg}"
        );
    }

    #[test]
    fn test_template_content_error_contains_relative_path() {
        let answers = json!({});
        let (template_root, _output_root, processor) = new_test_processor(answers);

        let nested_dir = template_root.path().join("src").join("templates");
        std::fs::create_dir_all(&nested_dir).unwrap();

        // Create a template file with valid name but invalid syntax (unclosed for loop)
        let file_path = nested_dir.join("mytemplate.baker.j2");
        let mut temp_file = File::create(&file_path).unwrap();
        // Use invalid template syntax (unclosed for loop)
        temp_file.write_all(b"{% for item in items %}{{ item }}").unwrap();

        // Process should fail because of invalid template syntax
        let result = processor.process(file_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // Error message should reference the template by its relative path
        assert!(
            err_msg.contains("src/templates/mytemplate.baker.j2")
                || err_msg.contains("src\\templates\\mytemplate.baker.j2"),
            "Error should reference the template file, got: {err_msg}"
        );
    }
}
