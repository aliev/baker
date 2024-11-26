use crate::{
    error::{BakerError, BakerResult},
    template::TemplateEngine,
};
use indexmap::IndexMap;

// Reads bakerfile and returns its content.
pub fn load_config<P: AsRef<std::path::Path>>(config_path: P) -> BakerResult<String> {
    let config_path = config_path.as_ref();
    if !config_path.exists() || !config_path.is_file() {
        return Err(BakerError::ConfigError(format!(
            "Invalid configuration path: {}",
            config_path.display()
        )));
    }

    Ok(std::fs::read_to_string(config_path).map_err(BakerError::IoError)?)
}

fn process_config_value(
    value: &serde_json::Value,
    context: &serde_json::Value,
    engine: &Box<dyn TemplateEngine>,
) -> BakerResult<serde_json::Value> {
    match value {
        serde_json::Value::String(s) => {
            // Process string values as templates
            let processed = engine.render(s, context)?;
            Ok(serde_json::Value::String(processed))
        }
        serde_json::Value::Array(arr) => {
            // Process each array item
            let mut processed_arr = Vec::new();
            for item in arr {
                processed_arr.push(process_config_value(item, context, engine)?);
            }
            Ok(serde_json::Value::Array(processed_arr))
        }
        serde_json::Value::Object(obj) => {
            // Process each object field
            let mut processed_obj = serde_json::Map::new();
            for (k, v) in obj {
                processed_obj.insert(k.clone(), process_config_value(v, context, engine)?);
            }
            Ok(serde_json::Value::Object(processed_obj))
        }
        _ => Ok(value.clone()),
    }
}

// Reads the JSON from bakerfile and applies the template
pub fn parse_config(
    content: String,
    engine: &Box<dyn TemplateEngine>,
) -> BakerResult<IndexMap<String, serde_json::Value>> {
    let bakerfile_map: IndexMap<String, serde_json::Value> =
        serde_json::from_str(&content).map_err(|e| BakerError::ConfigError(e.to_string()))?;
    let mut processed_config = IndexMap::with_capacity(bakerfile_map.len());
    // Process in the original order while keeping track of processed values
    for (key, value) in bakerfile_map.iter() {
        let processed_value = if let serde_json::Value::String(s) = value {
            if !s.contains("{%") && !s.contains("{{") {
                // Non-templated strings can be added directly
                value.clone()
            } else {
                // Pass current processed state directly without "baker" wrapper
                let current_context = serde_json::to_value(&processed_config)
                    .map_err(|e| BakerError::ConfigError(e.to_string()))?;
                process_config_value(value, &current_context, engine)?
            }
        } else {
            // Process arrays and other types with current context
            let current_context = serde_json::to_value(&processed_config)
                .map_err(|e| BakerError::ConfigError(e.to_string()))?;
            process_config_value(value, &current_context, engine)?
        };

        processed_config.insert(key.clone(), processed_value);
    }

    Ok(processed_config)
}
