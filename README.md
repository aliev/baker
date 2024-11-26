# Baker - Project Scaffolding Tool

<div align="center">
  <img src="artwork/logo.ai.png" alt="" width=320>
  <p><strong>Baker: is a fast project scaffolding tool written in Rust that generates projects from minijinja templates.</strong></p>
</div>

## Features

Baker was inspired by the [cookiecutter](https://github.com/cookiecutter/cookiecutter) project. The key difference is that Baker does not rely on Python dependencies, operates much faster, and provides a unified binary. At the same time, it retains excellent integration capabilities with Jinja 2 (via [Minijinja](https://github.com/mitsuhiko/minijinja)).

Basic features

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

Files with the double extension `.j2` (the minijinja extension) will be processed by the template engine. For example, files with extensions like `main.py.j2` or even `.dockerignore.j2` (since these are effectively files with double extensions) will be processed and copied as `main.py` and `.dockerignore`, respectively.

You can leverage all the features of the template engine in file names, including conditions and filters, for example: `{% if baker.create_main_file %}main.py{% endif %}` will create a file only if `create_main_file` is true (answered as `yes`).

```
template/
├── baker.json           # Template configuration
├── .bakerignore         # Files to ignore (optional)
├── .dockerignore.j2     # The template file will be processed as `.dockerignore`
├── tests.py.j2          # The template file will be processed as `tests.py`
├── {% if create_main_file %}main.py{% endif %}
├── template.j2          # This file will not be processed but will be copied as is
├── hooks/               # Template hooks (optional)
│   ├── pre_gen_project
│   └── post_gen_project
└── ... other template files ...
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
{{ variable_name }}
```

## Security

- Hooks require explicit user confirmation before execution
- Use `--skip-hooks-check` to bypass confirmation

## Example

```bash
# Using a local template
baker ./my-template ./output
```
