use crate::error::{BakerError, BakerResult};
use indexmap::IndexMap;

// Applies content to template string using the template engine.
fn render_string(template_str: &str, context: &serde_json::Value) -> BakerResult<String> {
    todo!()
}

fn process_value(
    value: &serde_json::Value,
    context: &serde_json::Value,
) -> BakerResult<serde_json::Value> {
    match value {
        serde_json::Value::String(s) => {
            // Process string values as templates
            let processed = render_string(s, context)?;
            Ok(serde_json::Value::String(processed))
        }
        serde_json::Value::Array(arr) => {
            // Process each array item
            let mut processed_arr = Vec::new();
            for item in arr {
                processed_arr.push(process_value(item, context)?);
            }
            Ok(serde_json::Value::Array(processed_arr))
        }
        serde_json::Value::Object(obj) => {
            // Process each object field
            let mut processed_obj = serde_json::Map::new();
            for (k, v) in obj {
                processed_obj.insert(k.clone(), process_value(v, context)?);
            }
            Ok(serde_json::Value::Object(processed_obj))
        }
        _ => Ok(value.clone()),
    }
}

// Reads the JSON from bakerfile and applies the template
pub fn parse_config(content: String) -> BakerResult<IndexMap<String, serde_json::Value>> {
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
                // For templated strings, use current processed state
                let current_context = serde_json::json!({
                    "baker": &processed_config
                });
                process_value(value, &current_context)?
            }
        } else {
            // Process arrays and other types with current context
            let current_context = serde_json::json!({
                "baker": &processed_config
            });
            process_value(value, &current_context)?
        };

        processed_config.insert(key.clone(), processed_value);
    }

    Ok(processed_config)
}
