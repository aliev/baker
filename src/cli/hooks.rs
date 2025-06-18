use crate::{
    error::Result,
    hooks::{confirm_hook_execution, get_hook_files, run_hook},
    ioutils::read_from,
    renderer::TemplateRenderer,
};
use serde_json::json;
use std::{path::Path, process::ChildStdout};

/// Handles pre and post hook execution
pub struct HookManager<'a> {
    renderer: &'a dyn TemplateRenderer,
    skip_hook_confirms: bool,
}

impl<'a> HookManager<'a> {
    pub fn new(renderer: &'a dyn TemplateRenderer, skip_hook_confirms: bool) -> Self {
        Self {
            renderer,
            skip_hook_confirms,
        }
    }

    /// Determines if hooks should be executed and gets hook file paths
    pub fn prepare_hooks<P: AsRef<Path>>(
        &self,
        template_root: P,
        pre_hook_filename: &str,
        post_hook_filename: &str,
    ) -> Result<(bool, std::path::PathBuf, std::path::PathBuf)> {
        let execute_hooks = confirm_hook_execution(
            &template_root,
            self.skip_hook_confirms,
            pre_hook_filename,
            post_hook_filename,
        )?;

        let (pre_hook_file, post_hook_file) =
            get_hook_files(&template_root, pre_hook_filename, post_hook_filename);

        Ok((execute_hooks, pre_hook_file, post_hook_file))
    }

    /// Executes pre-generation hook if it exists
    pub fn execute_pre_hook<P: AsRef<Path>>(
        &self,
        template_root: &P,
        output_root: &P,
        pre_hook_file: &P,
        execute_hooks: bool,
    ) -> Result<Option<ChildStdout>> {
        if execute_hooks && pre_hook_file.as_ref().exists() {
            log::debug!("Executing pre-hook: {}", pre_hook_file.as_ref().display());
            run_hook(template_root, output_root, pre_hook_file, None)
        } else {
            Ok(None)
        }
    }

    /// Executes post-generation hook if it exists
    pub fn execute_post_hook<P: AsRef<Path>>(
        &self,
        template_root: &P,
        output_root: &P,
        post_hook_file: &P,
        execute_hooks: bool,
        answers: Option<&serde_json::Value>,
    ) -> Result<()> {
        if execute_hooks && post_hook_file.as_ref().exists() {
            log::debug!("Executing post-hook: {}", post_hook_file.as_ref().display());
            let post_hook_stdout = run_hook(template_root, output_root, post_hook_file, answers)?;

            if let Some(post_hook_stdout) = post_hook_stdout {
                let result = read_from(post_hook_stdout).unwrap_or_default();
                log::debug!("Post-hook stdout content: {}", result);
            }
        }
        Ok(())
    }

    /// Renders hook filenames using the template engine
    pub fn render_hook_filenames(
        &self,
        pre_hook_filename: &str,
        post_hook_filename: &str,
    ) -> Result<(String, String)> {
        let pre_hook_filename = self.renderer.render(pre_hook_filename, &json!({}))?;
        let post_hook_filename = self.renderer.render(post_hook_filename, &json!({}))?;
        Ok((pre_hook_filename, post_hook_filename))
    }
}
