use crate::{
    error::{BakerError, BakerResult},
    template::TemplateRenderer,
};
use indexmap::IndexMap;

// Reads bakerfile and returns its content.
pub fn config_file_read<P: AsRef<std::path::Path>>(bakerfile_path: P) -> BakerResult<String> {
    let bakerfile_path = bakerfile_path.as_ref();
    if !bakerfile_path.exists() || !bakerfile_path.is_file() {
        return Err(BakerError::ConfigError(format!(
            "Invalid configuration path: {}",
            bakerfile_path.display()
        )));
    }

    Ok(std::fs::read_to_string(bakerfile_path).map_err(BakerError::IoError)?)
}

fn config_parse_value(
    value: &serde_json::Value,
    context: &serde_json::Value,
    template_renderer: &Box<dyn TemplateRenderer>,
) -> BakerResult<serde_json::Value> {
    match value {
        serde_json::Value::String(s) => {
            // Process string values as templates
            let processed = template_renderer.render(s, context)?;
            Ok(serde_json::Value::String(processed))
        }
        serde_json::Value::Array(arr) => {
            // Process each array item
            let mut processed_arr = Vec::new();
            for item in arr {
                processed_arr.push(config_parse_value(item, context, template_renderer)?);
            }
            Ok(serde_json::Value::Array(processed_arr))
        }
        serde_json::Value::Object(obj) => {
            // Process each object field
            let mut processed_obj = serde_json::Map::new();
            for (k, v) in obj {
                processed_obj.insert(
                    k.clone(),
                    config_parse_value(v, context, template_renderer)?,
                );
            }
            Ok(serde_json::Value::Object(processed_obj))
        }
        _ => Ok(value.clone()),
    }
}

// Reads the JSON from bakerfile and applies the template
pub fn config_parse_content(
    content: String,
    template_renderer: &Box<dyn TemplateRenderer>,
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
                config_parse_value(value, &current_context, template_renderer)?
            }
        } else {
            // Process arrays and other types with current context
            let current_context = serde_json::to_value(&processed_config)
                .map_err(|e| BakerError::ConfigError(e.to_string()))?;
            config_parse_value(value, &current_context, template_renderer)?
        };

        processed_config.insert(key.clone(), processed_value);
    }

    Ok(processed_config)
}
