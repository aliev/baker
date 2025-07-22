//! Configuration loading and management

use crate::config::question::Question;
use crate::constants::{
    CONFIG_FILENAMES, DEFAULT_LOOP_CONTENT_SEPARATOR, DEFAULT_LOOP_SEPARATOR,
    DEFAULT_POST_HOOK, DEFAULT_PRE_HOOK, DEFAULT_TEMPLATE_SUFFIX,
};
use crate::error::{Error, Result};
use crate::ext::PathExt;
use indexmap::IndexMap;
use serde::Deserialize;
use std::path::Path;

/// Main configuration structure holding all questions
#[derive(Debug, Deserialize)]
pub struct ConfigV1 {
    #[serde(default = "get_default_template_suffix")]
    pub template_suffix: String,
    #[serde(default = "get_default_loop_separator")]
    pub loop_separator: String,
    #[serde(default = "get_default_loop_content_separator")]
    pub loop_content_separator: String,
    #[serde(default = "get_default_template_globs")]
    pub template_globs: Vec<String>,
    #[serde(default)]
    pub questions: IndexMap<String, Question>,
    #[serde(default = "get_default_post_hook_filename")]
    pub post_hook_filename: String,
    #[serde(default = "get_default_pre_hook_filename")]
    pub pre_hook_filename: String,
}

impl ConfigV1 {
    pub fn validate(&self) -> Result<(), Error> {
        if self.template_suffix.is_empty() {
            return Err(Error::ConfigValidation(
                "template_suffix must not be empty".into(),
            ));
        }
        if !self.template_suffix.starts_with('.') || self.template_suffix.len() < 2 {
            return Err(Error::ConfigValidation("template_suffix must start with '.' and have at least 1 character after it".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "schemaVersion")]
pub enum Config {
    #[serde(rename = "v1")]
    V1(ConfigV1),
}

impl Config {
    pub fn load_config<P: AsRef<Path>>(template_root: P) -> Result<Self> {
        let template_root = template_root.as_ref().to_path_buf();
        let template_dir = template_root.to_str_checked()?.to_string();

        for config_file_name in CONFIG_FILENAMES.iter() {
            let config_file_path = template_root.join(config_file_name);

            if config_file_path.exists() {
                let content = std::fs::read_to_string(config_file_path)?;
                let config: Config = match *config_file_name {
                    "baker.json" => serde_json::from_str(&content)?,
                    "baker.yaml" | "baker.yml" => serde_yaml::from_str(&content)?,
                    _ => unreachable!(),
                };

                return Ok(config);
            }
        }

        Err(Error::ConfigNotFound {
            template_dir,
            config_files: CONFIG_FILENAMES.join(", "),
        })
    }
}

fn get_default_template_suffix() -> String {
    DEFAULT_TEMPLATE_SUFFIX.to_string()
}

fn get_default_template_globs() -> Vec<String> {
    Vec::new()
}

fn get_default_post_hook_filename() -> String {
    DEFAULT_POST_HOOK.to_string()
}

fn get_default_pre_hook_filename() -> String {
    DEFAULT_PRE_HOOK.to_string()
}

fn get_default_loop_separator() -> String {
    DEFAULT_LOOP_SEPARATOR.to_string()
}

fn get_default_loop_content_separator() -> String {
    DEFAULT_LOOP_CONTENT_SEPARATOR.to_string()
}
