use crate::{
    error::{BakerError, BakerResult},
    render::TemplateRenderer,
};
use indexmap::IndexMap;

fn process_value(
    value: &serde_json::Value,
    context: &serde_json::Value,
    template_processor: &Box<dyn TemplateRenderer>,
) -> BakerResult<serde_json::Value> {
    match value {
        serde_json::Value::String(s) => {
            // Process string values as templates
            let processed = template_processor.render(s, context)?;
            Ok(serde_json::Value::String(processed))
        }
        serde_json::Value::Array(arr) => {
            // Process each array item
            let mut processed_arr = Vec::new();
            for item in arr {
                processed_arr.push(process_value(item, context, template_processor)?);
            }
            Ok(serde_json::Value::Array(processed_arr))
        }
        serde_json::Value::Object(obj) => {
            // Process each object field
            let mut processed_obj = serde_json::Map::new();
            for (k, v) in obj {
                processed_obj.insert(k.clone(), process_value(v, context, template_processor)?);
            }
            Ok(serde_json::Value::Object(processed_obj))
        }
        _ => Ok(value.clone()),
    }
}

// Reads the JSON from bakerfile and applies the template
pub fn parse_config(
    content: String,
    template_processor: &Box<dyn TemplateRenderer>,
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
                // For templated strings, use current processed state
                let current_context = serde_json::json!({
                    "baker": &processed_config
                });
                process_value(value, &current_context, template_processor)?
            }
        } else {
            // Process arrays and other types with current context
            let current_context = serde_json::json!({
                "baker": &processed_config
            });
            process_value(value, &current_context, template_processor)?
        };

        processed_config.insert(key.clone(), processed_value);
    }

    Ok(processed_config)
}
