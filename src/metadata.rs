use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct TemplateMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GenerationMetadata {
    #[serde(flatten)]
    pub template_metadata: TemplateMetadata,
    pub answers: serde_json::Value,
}

impl GenerationMetadata {
    pub fn new(template_metadata: TemplateMetadata, answers: serde_json::Value) -> Self {
        Self { template_metadata, answers }
    }

    pub fn save_to_file(&self, path: &PathBuf) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_to_file_filemetadata() {
        let file_metadata = TemplateMetadata {
            directory: Some("examples/demo".to_string()),
            ..Default::default()
        };
        let answers = serde_json::json!({"foo": 42, "bar": "baz"});
        let gen_metadata = GenerationMetadata::new(file_metadata, answers.clone());

        let tmpfile = NamedTempFile::new().unwrap();
        let path = tmpfile.path();

        gen_metadata.save_to_file(&path.to_path_buf()).unwrap();

        let contents = fs::read_to_string(path).unwrap();
        let expected = r#"{
  "directory": "examples/demo",
  "answers": {
    "bar": "baz",
    "foo": 42
  }
}"#;
        assert_eq!(contents, expected);
    }

    #[test]
    fn test_save_to_file_gitmetadata() {
        let git_metadata = TemplateMetadata {
            template_url: Some("https://github.com/example/repo.git".to_string()),
            branch: Some("main".to_string()),
            tag: Some("v1.0.0".to_string()),
            commit: Some("abcdef123456".to_string()),
            ..Default::default()
        };
        let answers = serde_json::json!({"foo": 42, "bar": "baz"});
        let gen_metadata = GenerationMetadata::new(git_metadata, answers.clone());

        let tmpfile = NamedTempFile::new().unwrap();
        let path = tmpfile.path();

        gen_metadata.save_to_file(&path.to_path_buf()).unwrap();

        let contents = std::fs::read_to_string(path).unwrap();
        let expected = r#"{
  "template_url": "https://github.com/example/repo.git",
  "branch": "main",
  "tag": "v1.0.0",
  "commit": "abcdef123456",
  "answers": {
    "bar": "baz",
    "foo": 42
  }
}"#;
        assert_eq!(contents, expected);
    }
}
