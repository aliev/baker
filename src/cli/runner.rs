use crate::{
    cli::{answers::AnswerCollector, processor::FileProcessor, Args, SkipConfirm},
    config::{Config, ConfigV1},
    error::{Error, Result},
    hooks::{confirm_hook_execution, get_hook_files, run_hook},
    ignore::parse_bakerignore_file,
    ioutils::path_to_str,
    loader::get_template,
    renderer::TemplateRenderer,
    template::{get_template_engine, processor::TemplateProcessor},
};
use globset::{Glob, GlobSet, GlobSetBuilder};
use log::debug;
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Main CLI runner that orchestrates the entire template generation workflow
pub struct Runner {
    args: Args,
}

impl Runner {
    pub fn new(args: Args) -> Self {
        Self { args }
    }

    /// Executes the complete template generation workflow
    pub fn run(self) -> Result<()> {
        let mut engine = get_template_engine();

        let output_root = self.get_output_dir(&self.args.output_dir, self.args.force)?;

        let template_root = get_template(
            self.args.template.as_str(),
            self.should_skip_overwrite_prompts(),
        )?;

        let config = self.load_and_validate_config(&template_root)?;

        self.add_templates_in_renderer(&template_root, &config, &mut engine);

        let (pre_hook_file, post_hook_file, execute_hooks) =
            self.setup_hooks(&template_root, &config, &engine)?;

        // Execute pre-generation hook
        let pre_hook_output = self.execute_pre_hook(
            &template_root,
            &output_root,
            &pre_hook_file,
            execute_hooks,
        )?;

        // Collect answers from all sources
        let answers = self.collect_answers(&config, &engine, pre_hook_output)?;

        // Process template files
        self.process_template_files(
            &template_root,
            &output_root,
            &config,
            &engine,
            &answers,
        )?;

        // Execute post-generation hook
        self.execute_post_hook(
            &template_root,
            &output_root,
            &post_hook_file,
            &answers,
            execute_hooks,
        )?;

        println!(
            "Template generation completed successfully in {}.",
            output_root.display()
        );
        Ok(())
    }

    /// Loads and validates the template configuration
    fn load_and_validate_config(
        &self,
        template_root: &PathBuf,
    ) -> Result<crate::config::ConfigV1> {
        let config = Config::load_config(template_root)?;
        let Config::V1(config) = config;
        config.validate()?;
        Ok(config)
    }

    /// Sets up pre and post hook files
    fn setup_hooks(
        &self,
        template_root: &PathBuf,
        config: &crate::config::ConfigV1,
        engine: &dyn crate::renderer::TemplateRenderer,
    ) -> Result<(PathBuf, PathBuf, bool)> {
        let pre_hook_filename = engine.render(
            &config.pre_hook_filename,
            &json!({}),
            Some(&config.pre_hook_filename),
        )?;
        let post_hook_filename = engine.render(
            &config.post_hook_filename,
            &json!({}),
            Some(&config.post_hook_filename),
        )?;

        let execute_hooks = confirm_hook_execution(
            template_root,
            self.should_skip_hook_prompts(),
            &pre_hook_filename,
            &post_hook_filename,
        )?;

        let (pre_hook_file, post_hook_file) =
            get_hook_files(template_root, &pre_hook_filename, &post_hook_filename);

        Ok((pre_hook_file, post_hook_file, execute_hooks))
    }

    /// Executes the pre-generation hook if it exists
    fn execute_pre_hook(
        &self,
        template_root: &PathBuf,
        output_root: &PathBuf,
        pre_hook_file: &PathBuf,
        execute_hooks: bool,
    ) -> Result<Option<String>> {
        if execute_hooks && pre_hook_file.exists() {
            log::debug!("Executing pre-hook: {}", pre_hook_file.display());
            run_hook(template_root, output_root, pre_hook_file, None)
        } else {
            Ok(None)
        }
    }

    /// Collects answers from all available sources
    fn collect_answers(
        &self,
        config: &crate::config::ConfigV1,
        engine: &dyn crate::renderer::TemplateRenderer,
        pre_hook_output: Option<String>,
    ) -> Result<serde_json::Value> {
        let collector = AnswerCollector::new(engine, self.args.non_interactive);
        collector.collect_answers(config, pre_hook_output, self.args.answers.clone())
    }

    /// Processes all template files
    fn process_template_files(
        &self,
        template_root: &PathBuf,
        output_root: &PathBuf,
        config: &crate::config::ConfigV1,
        engine: &dyn crate::renderer::TemplateRenderer,
        answers: &serde_json::Value,
    ) -> Result<()> {
        let bakerignore = parse_bakerignore_file(template_root)?;

        let processor = TemplateProcessor::new(
            engine,
            template_root.clone(),
            output_root.clone(),
            answers,
            &bakerignore,
            config.template_suffix.as_str(),
        );

        let file_processor = FileProcessor::new(processor, &self.args.skip_confirms);
        file_processor.process_all_files(template_root)
    }

    /// Executes the post-generation hook if it exists
    fn execute_post_hook(
        &self,
        template_root: &PathBuf,
        output_root: &PathBuf,
        post_hook_file: &PathBuf,
        answers: &serde_json::Value,
        execute_hooks: bool,
    ) -> Result<()> {
        if execute_hooks && post_hook_file.exists() {
            log::debug!("Executing post-hook: {}", post_hook_file.display());
            let post_hook_stdout =
                run_hook(template_root, output_root, post_hook_file, Some(answers))?;

            if let Some(result) = post_hook_stdout {
                log::debug!("Post-hook stdout content: {result}");
            }
        }
        Ok(())
    }

    /// Determines if overwrite prompts should be skipped
    fn should_skip_overwrite_prompts(&self) -> bool {
        self.args.skip_confirms.contains(&SkipConfirm::All)
            || self.args.skip_confirms.contains(&SkipConfirm::Overwrite)
    }

    /// Determines if hook execution prompts should be skipped
    fn should_skip_hook_prompts(&self) -> bool {
        self.args.skip_confirms.contains(&SkipConfirm::All)
            || self.args.skip_confirms.contains(&SkipConfirm::Hooks)
    }

    /// Ensures the output directory exists and is safe to write to.
    fn get_output_dir<P: AsRef<Path>>(
        &self,
        output_dir: P,
        force: bool,
    ) -> Result<PathBuf> {
        let output_dir = output_dir.as_ref();
        if output_dir.exists() && !force {
            return Err(Error::OutputDirectoryExistsError {
                output_dir: output_dir.display().to_string(),
            });
        }
        Ok(output_dir.to_path_buf())
    }

    /// Adds template files from a directory into a MiniJinja renderer, using multiple glob patterns.
    ///
    /// This function scans the `template_root` directory recursively and adds all files matching
    /// any of the glob patterns specified in `config.template_imports_patterns` to the provided
    /// template engine. This allows for flexible inclusion of templates with different extensions
    /// or naming conventions.
    ///
    /// # Arguments
    /// * `template_root` - The root directory containing template files.
    /// * `config` - The configuration object specifying glob patterns for template imports.
    /// * `engine` - The template renderer to which the templates will be added.
    ///
    /// Only files matching at least one of the provided patterns will be processed and added.
    pub fn add_templates_in_renderer(
        &self,
        template_root: &Path,
        config: &ConfigV1,
        engine: &mut dyn TemplateRenderer,
    ) {
        let templates_import_globset =
            self.build_templates_import_globset(template_root, &config.template_globs);

        if let Some(globset) = templates_import_globset {
            debug!("Adding templates from glob patterns: {:?}", &config.template_globs);
            WalkDir::new(template_root)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|entry| entry.path().is_file())
                .filter(|entry| globset.is_match(entry.path()))
                .filter_map(|entry| {
                    let path = entry.path();
                    let rel_path = path.strip_prefix(template_root).ok()?;
                    let rel_path_str = rel_path.to_str()?;
                    fs::read_to_string(path)
                        .ok()
                        .map(|content| (rel_path_str.to_owned(), content))
                })
                .for_each(|(filename, content)| {
                    debug!("Adding template: {filename}");
                    engine.add_template(&filename, &content).unwrap();
                });
        } else {
            debug!("template_imports_patters is empty. No patterns provided for adding templates in the template engine for import and include.");
        }
    }

    /// Constructs a `GlobSet` for matching template files using multiple patterns relative to a root directory.
    ///
    /// This function takes a list of glob patterns (such as `*.tpl` or `*.jinja`) and builds a `GlobSet`
    /// that can be used to efficiently match files within the `template_root` directory. Each pattern is
    /// joined with the `template_root` to ensure correct matching against absolute file paths.
    ///
    /// # Arguments
    /// * `template_root` - The root directory where template files are located.
    /// * `patterns` - A list of glob patterns (relative to `template_root`) to match template files.
    ///
    /// # Returns
    /// * `Some(GlobSet)` if at least one pattern is provided and the set is built successfully.
    /// * `None` if the pattern list is empty.
    ///
    pub fn build_templates_import_globset(
        &self,
        template_root: &Path,
        patterns: &Vec<String>,
    ) -> Option<GlobSet> {
        if patterns.is_empty() {
            return None;
        }
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let path_to_ignored_pattern = template_root.join(pattern);
            let path_str = path_to_str(&path_to_ignored_pattern).unwrap_or_else(|_| {
                debug!("Failed to convert path to string: {path_to_ignored_pattern:?}");
                ""
            });
            builder.add(Glob::new(path_str).unwrap());
        }
        Some(builder.build().unwrap())
    }
}

/// Main entry point for CLI execution
pub fn run(args: Args) -> Result<()> {
    let runner = Runner::new(args);
    runner.run()
}
