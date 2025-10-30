use crate::{
    cli::{
        answers::AnswerCollector, context::GenerationContext, hooks::run_hook,
        processor::FileProcessor, Args, SkipConfirm,
    },
    config::{Config, ConfigV1},
    error::{Error, Result},
    ignore::parse_bakerignore_file,
    loader::get_template,
    prompt::confirm,
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
        let mut context = self.prepare_environment(&mut engine)?;

        let hook_plan = self.prepare_hooks(&context, &engine)?;

        let pre_hook_output = self.maybe_run_pre_hook(&hook_plan, &context, &engine)?;

        let answers = self.gather_answers(
            context.config(),
            &engine,
            pre_hook_output,
            context.template_root(),
        )?;
        context.set_answers(answers);

        self.process_templates(&context, &engine)?;

        self.maybe_run_post_hook(&hook_plan, &context, &engine)?;

        self.finish(&context);

        Ok(())
    }

    fn prepare_environment(
        &self,
        engine: &mut dyn TemplateRenderer,
    ) -> Result<GenerationContext> {
        let output_root = self.prepare_output_dir()?;
        let template_root = self.resolve_template()?;
        let config = self.load_and_validate_config(&template_root)?;
        debug!("Loaded config: follow_symlinks={}", config.follow_symlinks);
        self.add_templates_in_renderer(&template_root, &config, engine);

        Ok(GenerationContext::new(
            template_root,
            output_root,
            config,
            self.args.skip_confirms.clone(),
            self.args.dry_run,
        ))
    }

    fn prepare_output_dir(&self) -> Result<PathBuf> {
        self.get_output_dir(&self.args.output_dir, self.args.force, self.args.dry_run)
    }

    fn resolve_template(&self) -> Result<PathBuf> {
        get_template(self.args.template.as_str(), self.should_skip_overwrite_prompts())
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

    fn prepare_hooks(
        &self,
        context: &GenerationContext,
        engine: &dyn crate::renderer::TemplateRenderer,
    ) -> Result<HookPlan> {
        let config = context.config();
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
        let pre_hook_runner = config.pre_hook_runner.clone();
        let post_hook_runner = config.post_hook_runner.clone();

        let execute_hooks = self.confirm_hook_execution(
            context.template_root(),
            self.should_skip_hook_prompts(),
            &pre_hook_filename,
            &post_hook_filename,
        )?;

        let (pre_hook_file, post_hook_file) = self.get_hook_files(
            context.template_root(),
            &pre_hook_filename,
            &post_hook_filename,
        );

        log::debug!(
            "Prepared hooks: pre={}, post={}, execute_hooks={}",
            pre_hook_file.display(),
            post_hook_file.display(),
            execute_hooks
        );

        Ok(HookPlan {
            pre_hook_file,
            post_hook_file,
            execute_hooks,
            pre_hook_runner,
            post_hook_runner,
        })
    }

    fn maybe_run_pre_hook(
        &self,
        hook_plan: &HookPlan,
        context: &GenerationContext,
        engine: &dyn TemplateRenderer,
    ) -> Result<Option<String>> {
        if !hook_plan.pre_hook_file.exists() {
            return Ok(None);
        }

        if context.dry_run() {
            log_dry_run_action("Would execute pre-hook", &hook_plan.pre_hook_file);
            return Ok(None);
        }

        if hook_plan.execute_hooks {
            let runner = render_hook_runner(
                engine,
                &hook_plan.pre_hook_runner,
                context.answers_opt(),
            )?;
            log::debug!("Executing pre-hook: {}", hook_plan.pre_hook_file.display());
            run_hook(
                context.template_root(),
                context.output_root(),
                &hook_plan.pre_hook_file,
                None,
                &runner,
            )
        } else {
            Ok(None)
        }
    }

    /// Collects answers from all available sources
    fn gather_answers(
        &self,
        config: &crate::config::ConfigV1,
        engine: &dyn crate::renderer::TemplateRenderer,
        pre_hook_output: Option<String>,
        template_root: &Path,
    ) -> Result<serde_json::Value> {
        let collector =
            AnswerCollector::new(engine, self.args.non_interactive, template_root);
        collector.collect_answers(config, pre_hook_output, self.args.answers.clone())
    }

    /// Processes all template files
    fn process_templates(
        &self,
        context: &GenerationContext,
        engine: &dyn crate::renderer::TemplateRenderer,
    ) -> Result<()> {
        let bakerignore = parse_bakerignore_file(context.template_root())?;

        let processor = TemplateProcessor::new(engine, context, &bakerignore);

        let file_processor = FileProcessor::new(processor, context);
        file_processor.process_all_files()
    }

    fn maybe_run_post_hook(
        &self,
        hook_plan: &HookPlan,
        context: &GenerationContext,
        engine: &dyn TemplateRenderer,
    ) -> Result<()> {
        if !hook_plan.post_hook_file.exists() {
            return Ok(());
        }

        if context.dry_run() {
            log_dry_run_action("Would execute post-hook", &hook_plan.post_hook_file);
            return Ok(());
        }

        if hook_plan.execute_hooks {
            let runner = render_hook_runner(
                engine,
                &hook_plan.post_hook_runner,
                context.answers_opt(),
            )?;
            log::debug!("Executing post-hook: {}", hook_plan.post_hook_file.display());
            let post_hook_stdout = run_hook(
                context.template_root(),
                context.output_root(),
                &hook_plan.post_hook_file,
                Some(context.answers()),
                &runner,
            )?;

            if let Some(result) = post_hook_stdout {
                log::debug!("Post-hook stdout content: {result}");
            }
        }
        Ok(())
    }

    fn finish(&self, context: &GenerationContext) {
        println!("{}", completion_message(context.dry_run(), context.output_root()));
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
        dry_run: bool,
    ) -> Result<PathBuf> {
        let output_dir = output_dir.as_ref();
        if output_dir.exists() && !force && !dry_run {
            return Err(Error::OutputDirectoryExistsError {
                output_dir: output_dir.display().to_string(),
            });
        }
        if dry_run {
            log_dry_run_action("Would use output directory", output_dir);
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
    fn add_templates_in_renderer(
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
                    if let Err(e) = engine.add_template(&filename, &content) {
                        log::warn!("Failed to add template {filename}: {e}");
                    }
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
    fn build_templates_import_globset(
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
            let path_str = path_to_ignored_pattern.display().to_string();
            if let Ok(glob) = Glob::new(&path_str) {
                builder.add(glob);
            } else {
                log::warn!("Invalid glob pattern: {path_str}");
            }
        }
        match builder.build() {
            Ok(globset) => Some(globset),
            Err(e) => {
                log::warn!("Failed to build glob set: {e}");
                None
            }
        }
    }

    fn confirm_hook_execution<P: AsRef<Path>>(
        &self,
        template_dir: P,
        skip_hooks_check: bool,
        pre_hook_filename: &str,
        post_hook_filename: &str,
    ) -> Result<bool> {
        let (pre_hook_file, post_hook_file) =
            self.get_hook_files(template_dir, pre_hook_filename, post_hook_filename);
        if pre_hook_file.exists() || post_hook_file.exists() {
            Ok(confirm(
            skip_hooks_check,
                format!(
                    "WARNING: This template contains the following hooks that will execute commands on your system:\n{}{}{}",
                    self.get_path_if_exists(&pre_hook_file),
                    self.get_path_if_exists(&post_hook_file),
                    "Do you want to run these hooks?",
                ),
            )?)
        } else {
            Ok(false)
        }
    }

    /// Gets paths to pre and post generation hook scripts.
    ///
    /// # Arguments
    /// * `template_dir` - Path to the template directory
    ///
    /// # Returns
    /// * `(PathBuf, PathBuf)` - Tuple containing paths to pre and post hook scripts
    fn get_hook_files<P: AsRef<Path>>(
        &self,
        template_dir: P,
        pre_hook_filename: &str,
        post_hook_filename: &str,
    ) -> (PathBuf, PathBuf) {
        let template_dir = template_dir.as_ref();
        let hooks_dir = template_dir.join("hooks");

        (hooks_dir.join(pre_hook_filename), hooks_dir.join(post_hook_filename))
    }

    /// Returns the file path as a string if the file exists; otherwise, returns an empty string.
    ///
    /// # Arguments
    /// * `path` - Path to the file
    ///
    /// # Returns
    /// * `String` - The file path or empty string
    fn get_path_if_exists<P: AsRef<Path>>(&self, path: P) -> String {
        let path = path.as_ref();
        if path.exists() {
            format!("{}\n", path.to_string_lossy())
        } else {
            String::new()
        }
    }
}

struct HookPlan {
    pre_hook_file: PathBuf,
    post_hook_file: PathBuf,
    execute_hooks: bool,
    pre_hook_runner: Vec<String>,
    post_hook_runner: Vec<String>,
}

fn render_hook_runner(
    engine: &dyn TemplateRenderer,
    runner_tokens: &[String],
    answers: Option<&serde_json::Value>,
) -> Result<Vec<String>> {
    let empty_answers = serde_json::Value::Object(Default::default());
    let answers_ref = answers.unwrap_or(&empty_answers);
    runner_tokens
        .iter()
        .map(|token| engine.render(token, answers_ref, Some("hook_runner")))
        .collect()
}

/// Emits a standardised dry-run log entry for the supplied action and filesystem target.
fn log_dry_run_action<A: AsRef<Path>>(action: &str, target: A) {
    log::info!("[DRY RUN] {}: {}", action, target.as_ref().display());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn base_args() -> Args {
        Args {
            template: "template".into(),
            output_dir: PathBuf::from("output"),
            force: false,
            verbose: 0,
            answers: None,
            skip_confirms: Vec::new(),
            non_interactive: false,
            dry_run: false,
        }
    }

    #[test]
    fn skip_flags_respect_overwrite_and_hook_prompts() {
        let mut args = base_args();
        args.skip_confirms = vec![SkipConfirm::Overwrite];
        let runner = Runner::new(args);
        assert!(runner.should_skip_overwrite_prompts());
        assert!(!runner.should_skip_hook_prompts());

        let mut args = base_args();
        args.skip_confirms = vec![SkipConfirm::Hooks];
        let runner = Runner::new(args);
        assert!(!runner.should_skip_overwrite_prompts());
        assert!(runner.should_skip_hook_prompts());
    }

    #[test]
    fn get_output_dir_errors_when_exists_without_force() {
        let temp_dir = TempDir::new().unwrap();
        let runner = Runner::new(base_args());
        let result = runner.get_output_dir(temp_dir.path(), false, false);
        match result {
            Err(Error::OutputDirectoryExistsError { output_dir }) => {
                assert!(output_dir.contains(temp_dir.path().to_str().unwrap()))
            }
            other => panic!("Expected OutputDirectoryExistsError, got {other:?}"),
        }
    }

    #[test]
    fn get_output_dir_allows_existing_when_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let runner = Runner::new(base_args());
        let result = runner.get_output_dir(temp_dir.path(), false, true).unwrap();
        assert_eq!(result, temp_dir.path());
    }

    #[test]
    fn build_templates_import_globset_matches_files() {
        let temp_dir = TempDir::new().unwrap();
        let mut args = base_args();
        args.template = temp_dir.path().to_string_lossy().into();
        let runner = Runner::new(args);
        let patterns = vec!["**/*.j2".to_string()];
        let globset =
            runner.build_templates_import_globset(temp_dir.path(), &patterns).unwrap();
        let file_path = temp_dir.path().join("example.j2");
        assert!(globset.is_match(&file_path));
    }

    #[test]
    fn build_templates_import_globset_returns_none_when_empty() {
        let temp_dir = TempDir::new().unwrap();
        let runner = Runner::new(base_args());
        let patterns: Vec<String> = Vec::new();
        assert!(runner
            .build_templates_import_globset(temp_dir.path(), &patterns)
            .is_none());
    }

    #[test]
    fn confirm_hook_execution_skips_prompt_when_flag_set() {
        let temp_dir = TempDir::new().unwrap();
        let hooks_dir = temp_dir.path().join("hooks");
        std::fs::create_dir_all(&hooks_dir).unwrap();
        std::fs::write(hooks_dir.join("pre"), "echo pre").unwrap();
        let runner = Runner::new(base_args());
        let execute_hooks =
            runner.confirm_hook_execution(temp_dir.path(), true, "pre", "post").unwrap();
        assert!(execute_hooks);
    }

    #[test]
    fn get_path_if_exists_returns_display_string() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        std::fs::write(&file_path, "content").unwrap();
        let runner = Runner::new(base_args());
        assert!(runner.get_path_if_exists(&file_path).contains("file.txt"));
        assert!(runner.get_path_if_exists(temp_dir.path().join("missing")).is_empty());
    }

    #[test]
    fn render_hook_runner_renders_tokens_with_answers() {
        let engine = crate::template::get_template_engine();
        let tokens = vec!["python{{ version }}".to_string(), "-u".to_string()];
        let answers = json!({ "version": "3" });

        let result = render_hook_runner(&engine, &tokens, Some(&answers)).unwrap();

        assert_eq!(result, vec!["python3".to_string(), "-u".to_string()]);
    }
}

/// Produces the user-facing completion string for the current run, accounting for dry-run mode.
fn completion_message(dry_run: bool, output_root: &Path) -> String {
    if dry_run {
        format!(
            "[DRY RUN] Template processing completed. No files were actually created in {}.",
            output_root.display()
        )
    } else {
        format!(
            "Template generation completed successfully in {}.",
            output_root.display()
        )
    }
}

/// Main entry point for CLI execution
pub fn run(args: Args) -> Result<()> {
    let runner = Runner::new(args);
    runner.run()
}
