use crate::error::{Error, Result};
use crate::ioutils::path_to_str;
use std::path::Path;
use super::{Config, CONFIG_LIST};

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load<P: AsRef<Path>>(template_root: P) -> Result<Config> {
        let template_root = template_root.as_ref().to_path_buf();
        let template_dir = path_to_str(&template_root)?.to_string();

        for config_file_name in CONFIG_LIST.iter() {
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

        Err(Error::ConfigNotFound { template_dir, config_files: CONFIG_LIST.join(", ") })
    }
}

impl Config {
    pub fn load_config<P: AsRef<Path>>(template_root: P) -> Result<Self> {
        ConfigLoader::load(template_root)
    }
}
