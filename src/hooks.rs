use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{BakerError, BakerResult};
use crate::prompt::read_input;

#[derive(Serialize)]
struct Output<'a> {
    template_dir: &'a str,
    output_dir: &'a str,
    context: &'a serde_json::Value,
}

pub fn get_hooks<P: AsRef<Path>>(template_dir: P) -> (PathBuf, PathBuf) {
    let template_dir = template_dir.as_ref();
    let pre_hook = template_dir.join("hooks").join("pre_gen_project");
    let post_hook = template_dir.join("hooks").join("post_gen_project");

    (pre_hook, post_hook)
}

pub fn confirm_hooks_execution(skip_hooks_check: bool) -> BakerResult<bool> {
    if skip_hooks_check {
        return Ok(true);
    }
    print!("WARNING: This template contains hooks that will execute commands on your system. Do you want to run these hooks? [y/N] ");
    let input = read_input()?;
    Ok(input.to_lowercase() == "y")
}

pub fn run_hook<P: AsRef<Path>>(
    template_dir: P,
    output_dir: P,
    script_path: P,
    context: &serde_json::Value,
) -> BakerResult<()> {
    let script_path = script_path.as_ref();

    let output = Output {
        template_dir: template_dir.as_ref().to_str().unwrap(),
        output_dir: output_dir.as_ref().to_str().unwrap(),
        context,
    };

    let output_data = serde_json::to_vec(&output).unwrap();

    if !script_path.exists() {
        return Ok(());
    }

    let mut child = Command::new(script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| BakerError::IoError(e))?;

    // Write context to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&output_data).map_err(BakerError::IoError)?;
    }

    // Wait for the process to complete
    let status = child.wait().map_err(BakerError::IoError)?;

    if !status.success() {
        return Err(BakerError::HookError(format!(
            "Hook failed with status: {}",
            status
        )));
    }

    Ok(())
}
