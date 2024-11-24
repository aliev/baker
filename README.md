# Baker - Project Template Generator

Baker is a powerful and flexible project scaffolding tool written in Rust that helps you generate projects from templates. It supports both local and GitHub templates with template processing capabilities.

## Features

- Template processing using Jinja2-like syntax
- Support for local and GitHub templates
- Interactive prompt for template variables
- Pre and post-generation hooks
- `.bakerignore` support for excluding files
- Template variable interpolation in filenames
- Configurable via `baker.json`

## Installation

To build from source:

```bash
cargo install --path .
```

## Usage

```bash
baker [OPTIONS] <TEMPLATE> <OUTPUT_DIR>
```

Arguments:

- `TEMPLATE`: Path to local template or GitHub repository (e.g., `user/repo`)
- `OUTPUT_DIR`: Directory where the generated project will be created

Options:

- `-f, --force`: Force overwrite existing output directory
- `-v, --verbose`: Enable verbose output
- `--skip-hooks-check`: Skip hooks safety check

## Template Structure

```
template/
├── baker.json           # Template configuration
├── .bakerignore         # Files to ignore (optional)
├── .dockerfile.j2       # The template file will be processed
├── main.py.j2           # The template file will be processed
├── template.j2          # This file will not be processed but just copied
├── hooks/               # Template hooks (optional)
│   ├── pre_gen_project
│   └── post_gen_project
└── ... template files ...
```

## Configuration

Create a `baker.json` file in your template root:

```json
{
  "project_name": "My Project",
  "use_docker": "no",
  "framework": ["Django", "Flask", "FastAPI"]
}
```

## Template Variables

Variables can be used in:

- File/directory names
- File contents
- Configuration values

Access variables in templates using:

```
{{ baker.variable_name }}
```

## Security

- Hooks require explicit user confirmation before execution
- Use `--skip-hooks-check` to bypass confirmation

## Example

```bash
# Using a local template
baker ./my-template ./output

# Using a GitHub template
baker username/repository ./output
```
