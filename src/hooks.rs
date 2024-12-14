//! Hook execution and management for Baker templates.
//! This module handles pre and post-generation hooks that allow templates
//! to execute custom scripts during project generation.

use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{ChildStdout, Command, Stdio};

use crate::error::{Error, Result};
use crate::prompt::prompt_confirm;

/// Structure representing data passed to hook scripts.
///
/// This data is serialized to JSON and passed to hook scripts via stdin.
#[derive(Serialize)]
pub struct Output<'a> {
    /// Absolute path to the template directory
    pub template_dir: &'a str,
    /// Absolute path to the output directory
    pub output_dir: &'a str,
    /// Context data for template rendering
    pub answers: Option<&'a serde_json::Value>,
}

/// Returns the file path as a string if the file exists; otherwise, returns an empty string.
/// # Arguments
/// * `path` - Path to the file
///
/// # Returns
/// * `String` - The file path
pub fn get_path_if_exists<P: AsRef<Path>>(path: P) -> String {
    let path = path.as_ref();
    if path.exists() {
        format!("{}\n", path.to_string_lossy())
    } else {
        "".into()
    }
}

/// Gets paths to pre and post generation hook scripts.
///
/// # Arguments
/// * `template_dir` - Path to the template directory
///
/// # Returns
/// * `(PathBuf, PathBuf)` - Tuple containing paths to pre and post hook scripts
pub fn get_hook_files<P: AsRef<Path>>(template_dir: P) -> (PathBuf, PathBuf) {
    let template_dir = template_dir.as_ref();
    let hooks_dir = template_dir.join("hooks");

    (hooks_dir.join("pre_gen_project"), hooks_dir.join("post_gen_project"))
}

/// Executes a hook script with the provided context.
///
/// # Arguments
/// * `template_dir` - Path to the template directory
/// * `output_dir` - Path to the output directory
/// * `script_path` - Path to the hook script to execute
/// * `context` - Template context data
///
/// # Returns
/// * `BakerResult<()>` - Success or error status of hook execution
///
/// # Notes
/// - Hook scripts receive context data as JSON via stdin
/// - Hooks must be executable files
/// - Non-zero exit codes from hooks are treated as errors
pub fn run_hook<P: AsRef<Path>>(
    template_dir: P,
    output_dir: P,
    script_path: P,
    answers: Option<&serde_json::Value>,
    is_piped_stdout: bool,
) -> Result<Option<ChildStdout>> {
    let script_path = script_path.as_ref();

    let output = Output {
        template_dir: template_dir.as_ref().to_str().unwrap(),
        output_dir: output_dir.as_ref().to_str().unwrap(),
        answers,
    };

    let output_data = serde_json::to_vec(&output).unwrap();

    if !script_path.exists() {
        return Ok(None);
    }

    let mut child = Command::new(script_path)
        .stdin(Stdio::piped())
        .stdout(if is_piped_stdout { Stdio::piped() } else { Stdio::inherit() })
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(Error::IoError)?;

    // Write context to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&output_data).map_err(Error::IoError)?;
        stdin.write_all(b"\n")?;
    }

    // Wait for the process to complete
    let status = child.wait().map_err(Error::IoError)?;

    if !status.success() {
        return Err(Error::HookExecutionError { status });
    }

    Ok(child.stdout)
}

pub fn confirm_hook_execution<P: AsRef<Path>>(
    template_dir: P,
    skip_hooks_confirmation: bool,
) -> Result<bool> {
    let (pre_hook_file, post_hook_file) = get_hook_files(template_dir);
    if pre_hook_file.exists() || post_hook_file.exists() {
        Ok(prompt_confirm(
            skip_hooks_confirmation,
                format!(
                    "WARNING: This template contains the following hooks that will execute commands on your system:\n{}{}{}",
                    get_path_if_exists(&pre_hook_file),
                    get_path_if_exists(&post_hook_file),
                    "Do you want to run these hooks?",
                ),
            )?)
    } else {
        Ok(false)
    }
}
