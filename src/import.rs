use crate::config::ConfigV1;
use crate::ioutils::path_to_str;
use crate::renderer::TemplateRenderer;
use globset::{Glob, GlobSet, GlobSetBuilder};
use log::debug;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Adds template files from a directory into a MiniJinja renderer, using multiple glob patterns.
///
/// This function scans the `template_root` directory recursively and adds all files matching
/// any of the glob patterns specified in `config.template_imports_patterns` to the provided
/// template engine. This allows for flexible inclusion of templates with different extensions
/// or naming conventions.
///
/// # Arguments
/// * `template_root` - The root directory containing template files.
/// * `config` - The configuration object specifying glob patterns for template imports.
/// * `engine` - The template renderer to which the templates will be added.
///
/// Only files matching at least one of the provided patterns will be processed and added.
pub fn add_templates_in_renderer(
    template_root: &Path,
    config: &ConfigV1,
    engine: &mut dyn TemplateRenderer,
) {
    let templates_import_globset =
        build_templates_import_globset(template_root, &config.template_globs);

    if let Some(globset) = templates_import_globset {
        debug!("Adding templates from glob patterns: {:?}", &config.template_globs);
        WalkDir::new(template_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|entry| entry.path().is_file())
            .filter(|entry| globset.is_match(entry.path()))
            .filter_map(|entry| {
                let path = entry.path();
                let rel_path = path.strip_prefix(template_root).ok()?;
                let rel_path_str = rel_path.to_str()?;
                fs::read_to_string(path)
                    .ok()
                    .map(|content| (rel_path_str.to_owned(), content))
            })
            .for_each(|(filename, content)| {
                debug!("Adding template: {}", filename);
                engine.add_template(&filename, &content).unwrap();
            });
    } else {
        debug!("template_imports_patters is empty. No patterns provided for adding templates in the template engine for import and include.");
    }
}

/// Constructs a `GlobSet` for matching template files using multiple patterns relative to a root directory.
///
/// This function takes a list of glob patterns (such as `*.tpl` or `*.jinja`) and builds a `GlobSet`
/// that can be used to efficiently match files within the `template_root` directory. Each pattern is
/// joined with the `template_root` to ensure correct matching against absolute file paths.
///
/// # Arguments
/// * `template_root` - The root directory where template files are located.
/// * `patterns` - A list of glob patterns (relative to `template_root`) to match template files.
///
/// # Returns
/// * `Some(GlobSet)` if at least one pattern is provided and the set is built successfully.
/// * `None` if the pattern list is empty.
///
pub fn build_templates_import_globset(
    template_root: &Path,
    patterns: &Vec<String>,
) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let path_to_ignored_pattern = template_root.join(pattern);
        let path_str = path_to_str(&path_to_ignored_pattern).unwrap_or_else(|_| {
            debug!("Failed to convert path to string: {:?}", path_to_ignored_pattern);
            ""
        });
        builder.add(Glob::new(path_str).unwrap());
    }
    Some(builder.build().unwrap())
}
