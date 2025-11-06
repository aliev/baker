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
    pub import_root: Option<String>,
    #[serde(default)]
    pub questions: IndexMap<String, Question>,
    #[serde(default = "get_default_post_hook_filename")]
    pub post_hook_filename: String,
    #[serde(default = "get_default_pre_hook_filename")]
    pub pre_hook_filename: String,
    #[serde(default = "get_default_post_hook_runner")]
    pub post_hook_runner: Vec<String>,
    #[serde(default = "get_default_pre_hook_runner")]
    pub pre_hook_runner: Vec<String>,
    #[serde(default = "get_default_follow_symlinks")]
    pub follow_symlinks: bool,
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

fn get_default_post_hook_runner() -> Vec<String> {
    Vec::new()
}

fn get_default_post_hook_filename() -> String {
    DEFAULT_POST_HOOK.to_string()
}

fn get_default_pre_hook_filename() -> String {
    DEFAULT_PRE_HOOK.to_string()
}

fn get_default_pre_hook_runner() -> Vec<String> {
    Vec::new()
}

fn get_default_loop_separator() -> String {
    DEFAULT_LOOP_SEPARATOR.to_string()
}

fn get_default_loop_content_separator() -> String {
    DEFAULT_LOOP_CONTENT_SEPARATOR.to_string()
}

fn get_default_follow_symlinks() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runners_default_to_empty_arrays() {
        let raw = r#"
schemaVersion: v1
pre_hook_filename: hooks/pre.sh
post_hook_filename: hooks/post.sh
questions: {}
"#;

        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;

        assert!(cfg.pre_hook_runner.is_empty());
        assert!(cfg.post_hook_runner.is_empty());
    }

    #[test]
    fn runners_parse_array_arguments() {
        let raw = r#"
schemaVersion: v1
pre_hook_filename: hooks/pre.ps1
pre_hook_runner: ["powershell", "-File"]
post_hook_filename: hooks/post.py
post_hook_runner: ["python3", "-u"]
questions: {}
"#;

        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;

        assert_eq!(
            cfg.pre_hook_runner,
            vec!["powershell".to_string(), "-File".to_string()]
        );
        assert_eq!(cfg.post_hook_runner, vec!["python3".to_string(), "-u".to_string()]);
    }

    #[test]
    fn follow_symlinks_defaults_false() {
        let raw = r#"schemaVersion: v1
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert!(!cfg.follow_symlinks);
    }

    #[test]
    fn follow_symlinks_parses_true() {
        let raw = r#"schemaVersion: v1
follow_symlinks: true
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert!(cfg.follow_symlinks);
    }

    #[test]
    fn import_root_defaults_to_none() {
        let raw = r#"schemaVersion: v1
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert!(cfg.import_root.is_none());
    }

    #[test]
    fn import_root_parses_path() {
        let raw = r#"schemaVersion: v1
import_root: "templates/shared"
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert_eq!(cfg.import_root, Some("templates/shared".to_string()));
    }

    #[test]
    fn import_root_parses_absolute_path() {
        let raw = r#"schemaVersion: v1
import_root: "/usr/local/templates"
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert_eq!(cfg.import_root, Some("/usr/local/templates".to_string()));
    }
}
