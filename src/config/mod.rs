pub mod loader;
pub mod question;
pub mod validation;

pub use loader::ConfigLoader;
pub use question::{
    IntoQuestionType, Question, QuestionRendered, QuestionType, Secret, Type,
};
pub use validation::Validation;

use indexmap::IndexMap;
use serde::Deserialize;

pub const CONFIG_LIST: &[&str] = &["baker.json", "baker.yaml", "baker.yml"];

/// Main configuration structure holding all questions
#[derive(Debug, Deserialize)]
pub struct ConfigV1 {
    #[serde(default)]
    pub questions: IndexMap<String, Question>,
    #[serde(default = "get_default_post_hook_filename")]
    pub post_hook_filename: String,
    #[serde(default = "get_default_pre_hook_filename")]
    pub pre_hook_filename: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "schemaVersion")]
pub enum Config {
    #[serde(rename = "v1")]
    V1(ConfigV1),
}

fn get_default_post_hook_filename() -> String {
    "post".to_string()
}

fn get_default_pre_hook_filename() -> String {
    "pre".to_string()
}
