# Import Root Example

This example demonstrates the `import_root` configuration feature in Baker.

## Overview

The `import_root` configuration allows you to specify a custom directory for template imports and macros, separate from the main template directory.

## Configuration

In `baker.yaml`:

```yaml
schemaVersion: v1
import_root: "shared_templates"
template_globs:
  - "*.jinja"
```

This tells Baker to look for importable templates (like macros) in the `shared_templates` directory instead of the template root.

## Usage

The template file `README.md.baker.j2` imports macros from `shared_templates/macros.jinja`:

```jinja
{% import "macros.jinja" as macros -%}
# {{ project_name }}

{{ macros.greeting(project_name) }}
```

## Benefits

- **Organization**: Keep reusable templates separate from the main template files
- **Shared Libraries**: Use an absolute path to share templates across multiple Baker templates
- **Flexibility**: Import root can be relative (to the template directory) or absolute

## Example

Run the example:

```bash
baker examples/import_root -o /tmp/test-output
```

