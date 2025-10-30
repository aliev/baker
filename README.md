# What is Baker?

<div align="center">
  <img src="assets/logo.ai.png" alt="" width=100>
  <p><strong>Baker</strong> is a command-line tool that helps you quickly <a href="https://en.wikipedia.org/wiki/Scaffold_(programming)">scaffold</a> new projects. It supports language-independent <a href="#using-hooks">hooks</a> for automating routine tasks. Baker is written in Rust and distributed as a standalone binary. Precompiled binaries are available for popular platforms on the <a href="https://github.com/aliev/baker/releases">releases page</a>.</p>
</div>

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Installation](#installation)
- [Project template example](#project-template-example)
  - [baker.yaml File](#bakeryaml-file)
  - [Files with .baker.j2 extension](#files-with-bakerj2-extension)
  - [Templated File Names](#templated-file-names)
  - [.bakerignore File](#bakerignore-file)
  - [Importing Jinja templates and macros](#importing-jinja-templates-and-macros)
  - [Loop Templates and Delimiters](#loop-templates-and-delimiters)
- [Recipes](#recipes)
  - [Passing Default Answers](#passing-default-answers)
  - [Non-Interactive Mode](#non-interactive-mode)
  - [Conditional Questions](#conditional-questions)
  - [Debugging Templates](#debugging-templates)
- [Hooks](#hooks)
  - [Customizing Hook Filenames](#customizing-hook-filenames)
  - [Customizing Hook Runners](#customizing-hook-runners)
  - [Available Platform Variables](#available-platform-variables)
- [Questions](#questions)
  - [Single-Input](#single-input)
  - [Yes / No](#yes--no)
  - [Single Choice](#single-choice)
  - [Multiple Choice](#multiple-choice)
  - [JSON Complex Type](#json-complex-type)
  - [YAML Complex Type](#yaml-complex-type)
  - [Validation](#validation)
    - [Required Field Validation](#required-field-validation)
    - [Numeric Value Validation](#numeric-value-validation)
    - [Pattern Matching with Regular Expressions](#pattern-matching-with-regular-expressions)
  - [Conditional questions](#conditional-questions)
- [Built-in Filters](#built-in-filters)
- [Comparing Baker to other project generators](#comparing-baker-to-other-project-generators)
- [Community Templates](#community-templates)

## Architecture Overview

Looking for an end-to-end view of the codebase? See [`docs/architecture.md`](docs/architecture.md) for a tour of the CLI runner, configuration loader, prompt layer, template processor, and supporting modules.

## Installation

You can install Baker using one of the following methods:

### Install via Scoop (Windows)

```ps1
scoop bucket add baker https://github.com/Gasoid/baker-scoop
scoop install baker/baker
```

### Install via Homebrew (macOS)

```bash
brew install aliev/tap/baker
```

### Install prebuilt binaries via shell script (Linux/macOS)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aliev/baker/releases/latest/download/baker-installer.sh | sh
```

### Install prebuilt binaries via PowerShell script (Windows)

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/aliev/baker/releases/latest/download/baker-installer.ps1 | iex"
```

Prebuilt binaries for all supported platforms are available on the [releases page](https://github.com/aliev/baker/releases).

## Project template example

To get started, you can use the [examples/demo](examples/demo) template, which demonstrates the core features of Baker:

```
│   Template configuration.
├── baker.yaml
│
│   The content of files with the `.baker.j2` extension will be processed by the templating engine
├── CONTRIBUTING.md.baker.j2
│
│   any other files will be copied as is,
├── README.md
│
│   unless they are listed in .bakerignore.
├── .bakerignore
│
│   File names can be templated
├── {{project_slug}}
│   └── __init__.py
│
│   any template features can be used, such as conditions:
└── {% if use_tests %}tests{% endif %}
    └── __init__.py
```

Note: By default Baker preserves symbolic links. To copy the contents the link points to (dereference), set `follow_symlinks: true` in `baker.yaml`:

```yaml
schemaVersion: v1
follow_symlinks: true
questions: {}
```
If a symlink points to a file it will be duplicated as a regular file. Symlinks to directories are currently recreated as symlinks (not recursively copied).

As a quick start, you can run the following command to generate a project:

```
baker examples/demo my-project
```

Each component of this template is described in detail below.

### `baker.yaml` File

The `baker.yaml` file defines the directory as a template. It contains template settings and [questions](#question) to be prompted to the user:

```yaml
schemaVersion: v1

questions:
  project_name:
    type: str
    help: Please enter the name of your project

  project_author:
    type: str
    help: "Please enter the author's name for {{project_name}}"

  project_slug:
    type: str
    help: Please enter the project slug (or press Enter to use the default)
    default: "{{ project_name|lower|replace(' ', '_') }}"

  use_tests:
    type: bool
    help: Will your project include tests?
    default: true
```

The values of the `help` and `default` keys can include templates for value substitution. Each subsequent question has access to the answers of the previous ones as demonstrated in `project_author` and `project_slug`.

In addition to YAML, Baker also supports JSON due to its backward compatibility with JSON. If multiple configuration files exist in the template directory, Baker will load them in the following order of priority: `baker.json`, `baker.yaml`, and `baker.yml`.

### Files with `.baker.j2` extension

The content of files with the `.baker.j2` extension will be processed by the templating engine and written to the target directory. The resulting files in the target directory will not include the `.baker.j2` extension in their names.

The content of such files can include the `questions` keys, which will be replaced by the corresponding user-provided answers during processing. Baker uses the MiniJinja this purpose. For more details on the syntax and capabilities of the templating engine, please refer to the [MiniJinja](https://docs.rs/minijinja/latest/minijinja/) documentation.

**Example:**

**Content of CONTRIBUTING.md.baker.j2**

```
  # {{project_name}}
  Author: {{project_author}}
```

**Processed file in target directory: CONTRIBUTING.md**

```
Content of CONTRIBUTING.md:
  # MyAwesomeProject
  Author: John Doe
```

Note:
The template suffix (default: .baker.j2) is fully configurable in your baker.yaml file using the template_suffix option. You can set it to any value, as long as it starts with a . and has at least one character after the dot (e.g., .tpl, .jinja, .tmpl). This allows you to use custom extensions for your template files.

Example:

```
schemaVersion: v1
template_suffix: ".tpl"
```

With this configuration, files ending with .tpl will be processed as templates instead of .baker.j2.

### Templated File Names

File and directory names can be templated to dynamically adjust based on user input.

**Example:**

```yaml
project_name:
  type: str
  help: Please enter the name of your project

project_slug:
  type: str
  help: Please enter the project slug (or press Enter to use the default)
  default: "{{ project_name|lower|replace(' ', '_') }}"
```

```
├── {{project_slug}}
│ └── __init__.py
```

This will create a directory named according to the value of the `project_slug` provided by the user.

---

File and directory names can include conditions that control their creation. If a condition evaluates to `false`, the corresponding file or directory will not be created. This feature is especially useful with [Yes / No](#yes--no) type questions, allowing you to dynamically include or exclude specific files and directories based on user responses.

**Example:**

```yaml
use_tests:
  type: bool
  help: Will your project include tests?
  default: true
```

```
└── {% if use_tests %}tests{% endif %}
    └── __init__.py
```

In this example, if the user answers "no" the `tests` directory will not be created.

### `.bakerignore` File

The `.bakerignore` file in the template root is used to exclude files and directories from being copied from the template. Bakerignore uses [Globset syntax](https://docs.rs/globset/latest/globset/#syntax).

By default, Baker ignores the following files and patterns:

```rust
const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    ".git/**",
    ".git",
    ".hg/**",
    ".hg",
    ".svn/**",
    ".svn",
    "**/.DS_Store",
    ".bakerignore",
    "hooks",
    "hooks/**",
    "baker.yaml",
    "baker.yml",
    "baker.json",
];
```

## Importing Jinja templates and macros

You can specify multiple patterns for files to be included in the template engine. Then you can [include templates](https://docs.rs/minijinja/latest/minijinja/syntax/index.html#-include-) or [import macros](https://docs.rs/minijinja/latest/minijinja/syntax/index.html#-import-) in your templates.

#### Example:

```yaml
schemaVersion: v1
template_globs:
  - "*.tpl"
  - "*.jinja"
questions:
  project_name:
    type: str
    help: Please enter the name of your project
```

This will include all files ending with .tpl and .jinja in the template engine, allowing you to use them in your templates.


## Loop Templates and Delimiters

Baker supports loop templates using MiniJinja for-loop blocks in template filenames. This allows you to generate multiple files based on a list of items in your answers.

For example, a template file named:

```
{% for item in items %}{{ item.name }}.md.baker.j2{% endfor %}
```
will generate a file for each item in the `items` array, with the filename rendered from `item.name`.

### loop_separator and loop_content_separator

When rendering loop templates, Baker uses two configuration options to split and organize the generated content:

- `loop_separator`: A string used to separate each file's content in the rendered output. This allows Baker to distinguish between multiple files generated from a single loop template. Default value is `<--SPLIT-->`.
- `loop_content_separator`: A string used to separate the filename from the file content within each split section. This enables Baker to extract the correct filename and its corresponding content. Default value is `<<CONTENT>>`.

**Example Usage:**

Suppose your template renders the following output:

```
filename1.md<<CONTENT>>Content for file 1<--SPLIT-->
filename2.md<<CONTENT>>Content for file 2<--SPLIT-->
```

Here, `<<CONTENT>>` is the `loop_content_separator` and `<--SPLIT-->` is the `loop_separator`. Baker will split the output on `<--SPLIT-->`, then split each part on `<<CONTENT>>` to get the filename and content for each file.

You can configure these separators in your Baker settings or pass them to the processor:

```yaml
schemaVersion: v1
template_suffix: ".baker.j2",
loop_separator: "<--SPLIT-->",
loop_content_separator: "<<CONTENT>>",
```

This mechanism allows flexible generation of multiple files from a single template, especially useful for code generation, documentation, or any batch file creation scenario.

## Recipes

### Passing Default Answers

Passing default answers can be useful when the answers are already known, such as in a CI/CD pipeline.

Default answers can be provided using the `--answers` option.

**Example**

```bash
# Alternatively, use --answers='{"name": "John"}'
echo '{"name": "John"}' | baker template my-project --answers=-
```

```yaml
schemaVersion: v1
questions:
  name:
    type: str
    help: What is your name?
```

The provided answer will be used as the default in the user prompt:

```
What is your name? [John]:
```

#### Non-Interactive Mode

For fully automated workflows like CI/CD pipelines, you can combine `--answers` with the `--non-interactive` flag to completely skip all prompts:

```bash
baker template my-project --answers='{"project_name": "Example Project"}' --non-interactive
```

In `--non-interactive` mode, Baker determines whether to skip user prompts based on two factors:

1. The `--non-interactive` flag itself
2. The template's `ask_if` conditions (if defined)

When a prompt is skipped, Baker uses the following strategy to determine the answer:

1. If an answer was already provided via the `--answers` parameter, use that value
2. If a default value (`default`) exists in the template configuration, use that
3. If neither exists, Baker will still prompt the user interactively for that question

For example, if your template contains:

```yaml
schemaVersion: v1
questions:
  project_name:
    type: str
    help: Please enter the name of your project
  project_author:
    type: str
    help: Please enter the author's name
    default: Anonymous
  use_tests:
    type: bool
    help: Will your project include tests?
    default: true
```

And you run:

```bash
baker template my-project --answers='{"project_name": "Example"}' --non-interactive
```

Baker will automatically use "Example" for `project_name`, "Anonymous" for `project_author` (from the default value), and `true` for `use_tests` (from the default value).

This is especially useful for CI/CD environments where interactive input isn't possible.

#### Conditional Questions

To skip the prompt entirely, you can use the `ask_if` attribute:

```yaml
schemaVersion: v1
questions:
  name:
    type: str
    help: What is your name?
    # Skips the prompt if "name" was provided in answers
    ask_if: name is not defined or name == ''
```

A detailed description of the `ask_if` key can be found in the [Conditional Questions](#conditional-questions) section.

### Debugging Templates

Since Baker uses MiniJinja, it benefits from all MiniJinja features, including debugging. You can use the `debug()` function to inspect the current context.

**Example**

```yaml
schemaVersion: v1
questions:
  first_name:
    type: str
    help: What is your name?
  last_name:
    type: str
    help: "Hello, {{first_name}}. What is your last name?"
  debug:
    type: str
    help: "{{debug()}}"
```

When you run the template, the `debug()` function will output the current context:

```
baker example out
What is your name?: aaa
Hello, aaa. What is your last name?: bbb
State {
    name: "temp",
    current_block: None,
    auto_escape: None,
    ctx: {
        "first_name": "aaa",
        "last_name": "bbb",
    },
    env: Environment {
        globals: {
            "debug": minijinja::functions::builtins::debug,
            "dict": minijinja::functions::builtins::dict,
            "namespace": minijinja::functions::builtins::namespace,
            "range": minijinja::functions::builtins::range,
        },
        tests: [
            "!=",
            "<",
            "<=",
            "==",
            ">",
            ">=",
            "boolean",
            "defined",
            "divisibleby",
...
```

This output provides a detailed view of the current context, including defined variables, their values, and available functions, helping you troubleshoot and debug your templates effectively.

## Hooks

Hooks are useful for performing routine tasks before (pre-hook) or after (post-hook) project generation.

Baker executes hooks as separate processes, which makes them language-independent.

For a hook to be executed, it must meet two requirements:

1. It must be located in the template directory `template_root/hooks/` and named according to the `pre_hook_filename` or `post_hook_filename` specified in the configuration.
2. It must be an executable file (`chmod +x template_root/hooks/<hook_filename>`).

When generating a project containing a hook, Baker will issue a warning:

```
baker examples/hooks out
WARNING: This template contains the following hooks that will execute commands on your system:
examples/hooks/hooks/post
Do you want to run these hooks? [y/N]
```

This warning can be omitted by using the `--skip-confirms=hooks` parameter.

The `pre` hook can generate answers and pass them to `baker` through `stdout`:

```python
#!/usr/bin/env python
import json

if __name__ == "__main__":
    # Passing the default answers to baker
    json.dump({"name": "John"}, sys.stdout)
```

The `post` hook can consume the answers, which will be passed by `baker` to the `stdin` of the `post` hook. The answers can be parsed as follows:

```python
#!/usr/bin/env python
import json
import pathlib
from typing import Any, TypedDict

path = pathlib.Path()

class Input(TypedDict):
    answers: dict[str, Any]
    template_dir: str
    output_dir: str

if __name__ == "__main__":
    context: Input = json.load(sys.stdin)
    output_dir_path = path / context["output_dir"]
    template_dir_path = path / context["template_dir"]
```

The diagram below illustrates this process in more detail

```mermaid
graph LR
    %% Data streams
    Pre[hooks/pre] --> stdout1[stdout]
    stdout1 --> |JSON answers| Baker[baker]
    any_cmd --> |JSON answers| stdin2[stdin] --> Baker
    Baker --> |JSON output| stdin3[stdin]
    stdin3 --> Post[hooks/post]
    Post --> stdout3[stdout]

    %% Add descriptions
    subgraph Pre-processing
        Pre
    end

    subgraph Main Process
        Baker
    end

    subgraph Post-processing
        Post
    end

    %% Style
    classDef process fill:#2d3436,stroke:#fff,stroke-width:2px,color:#fff
    classDef stream fill:#3498db,stroke:#fff,stroke-width:2px,color:#fff

    class Pre,Post,Baker process
    class stdin2,stdin3,stdout1,stdout3 stream
```

### Customizing Hook Filenames

By default, Baker looks for hook scripts named `pre` and `post` in the `hooks` directory of your template. You can customize these filenames using the `pre_hook_filename` and `post_hook_filename` configuration options in your `baker.yaml` file:

```yaml
schemaVersion: v1

questions:
  # Your regular questions here...

# Custom hook filenames
pre_hook_filename: "setup-environment"
post_hook_filename: "finalize-project"
```

With this configuration, Baker will:

1. Look for a pre-hook script at `template_root/hooks/setup-environment`
2. Look for a post-hook script at `template_root/hooks/finalize-project`

Hook filenames also support template strings, which can be used to create platform-specific hooks:

```yaml
schemaVersion: v1

questions:
  license:
    type: str
    help: "Please select a licence for {{platform.os}}"
    default: MIT
    choices:
      - MIT
      - BSD
      - GPLv3
      - Apache Software License 2.0
      - Not open source

pre_hook_filename: "{{platform.family}}/pre"
post_hook_filename: "{{platform.family}}/post"
```

This configuration allows you to organize hooks by platform. For example:

```
hooks/
├── unix/
│   ├── pre
│   └── post
└── windows/
    ├── pre
    └── post
```

Baker will automatically select the appropriate hook based on the current platform.

### Customizing Hook Runners

You can declare how Baker should execute hook scripts by specifying a runner for
each hook. Runners are defined as arrays of strings (similar to Docker's
`ENTRYPOINT` syntax), allowing you to include the command and its arguments.

```yaml
pre_hook_filename: pre.ps1
pre_hook_runner:
  - powershell
  - -NoLogo
  - -File

post_hook_filename: post.py
post_hook_runner:
  - python3
  - -u
```

- If a runner is provided, Baker runs the hook using the supplied command and
  passes the rendered hook path as the final argument.
- If omitted (default), Baker executes the hook path directly, matching the
  existing behaviour on Unix systems with executable scripts.
- Runner tokens are rendered through the template engine, so you can use
  variables from `baker.yaml` if needed.

This feature is especially helpful on Windows where scripts such as `.ps1`
cannot be launched directly, and on Unix when you want to force a specific
interpreter (e.g., Python, Node.js, Bash).

### Available Platform Variables

Baker provides these platform variables that can be used in templates and hook filenames:

- `platform.os` - Operating system name (e.g., "linux", "macos", "windows")
- `platform.family` - OS family (e.g., "unix", "windows")
- `platform.arch` - CPU architecture (e.g., "x86_64", "aarch64")

You can use these variables in any template, including hook filenames, questions, help text, defaults, etc.

## Questions

Baker supports various question components, which are described below.

### Single-Input

Single Input prompts the user to enter a text value.

#### Example

```yaml
schemaVersion: v1

questions:
  readme_content:
    type: str
    help: Please enter the content for CONTRIBUTING.md
    default: My super duper project
```

- **`type`**: Must be `str`.
- **`help`**: Should be a string, optionally containing a `minijinja` template.
- **`default`**: Should be a string, optionally containing a `minijinja` template.

#### Result

```
Please enter the content for CONTRIBUTING.md []:
```

### Yes / No

#### Example

```yaml
schemaVersion: v1

questions:
  include_tests:
    type: bool
    help: Do you want to include tests in the generated project?
    default: true
```

- **`type`**: Must be `bool`.
- **`help`**: Should be a string, optionally containing a `minijinja` template.
- **`default`**: Should be a boolean value, defaulting to `false`.

#### Result

```
Do you want to include tests in the generated project? [Y/n]
```

### Single Choice

#### Example

```yaml
schemaVersion: v1

questions:
  favourite_language:
    type: str
    help: What is your favorite programming language?
    default: Rust
    choices:
      - Python
      - Rust
      - Go
      - TypeScript
```

- **`type`**: Must be `str`.
- **`help`**: Should be a string, optionally containing a `minijinja` template.
- **`choices`**: Should be a list of strings.
- **`default`**: Should be a string, optionally containing a `minijinja` template.

#### Result

```
What is your favorite programming language?:
  Python
> Rust
  Go
  TypeScript
```

### Multiple Choice

#### Example

```yaml
schemaVersion: v1

questions:
  favourite_language:
    type: str
    help: What are your favorite programming languages?
    multiselect: true
    default:
      - Python
      - Rust
    choices:
      - Python
      - Rust
      - Go
      - TypeScript
```

- **`type`**: Must be `str`.
- **`help`**: Should be a string, optionally containing a `minijinja` template.
- **`multiselect`**: Must be `true` to enable multiple choice.
- **`default`**: Should be a list of strings.
- **`choices`**: Should be a list of strings.

#### Result

```
What are your favorite programming languages?:
  [x] Python
> [x] Rust
  [ ] Go
  [ ] TypeScript
```

### JSON Complex Type

The JSON type allows you to collect structured data from the user in JSON format. This is useful for configuration files, environment settings, and other structured data.

#### Example

```yaml
schemaVersion: v1

questions:
  database_config:
    type: json
    help: Configure your database settings
    schema: |
      {
        "type": "object",
        "required": ["engine", "host", "port"],
        "properties": {
          "engine": {
            "type": "string",
            "enum": ["postgresql", "mysql", "sqlite", "mongodb"]
          },
          "host": {
            "type": "string"
          },
          "port": {
            "type": "integer",
            "minimum": 1,
            "maximum": 65535
          }
        }
      }
    default: |
      {
        "engine": "postgresql",
        "host": "localhost",
        "port": 5432
      }
```

- **`type`**: Must be `json`.
- **`help`**: Should be a string, optionally containing a `minijinja` template.
- **`schema`**: Optional JSON Schema for validation. Follows the [JSON Schema standard](https://json-schema.org/).
- **`schema_file`**: Optional path to an external JSON Schema file (relative to template root). Takes precedence over inline `schema`.
- **`default`**: JSON object, can be provided as a string or native YAML object.

#### Loading Schema from External File

For better organization and reusability, you can store your JSON Schema in a separate file:

```yaml
schemaVersion: v1

questions:
  database_config:
    type: json
    help: Configure your database settings
    schema_file: database.schema.json
    default: |
      {
        "engine": "postgresql",
        "host": "localhost",
        "port": 5432
      }
```

**database.schema.json:**
```json
{
  "type": "object",
  "required": ["engine", "host", "port"],
  "properties": {
    "engine": {
      "type": "string",
      "enum": ["postgresql", "mysql", "sqlite", "mongodb"]
    },
    "host": {
      "type": "string"
    },
    "port": {
      "type": "integer",
      "minimum": 1,
      "maximum": 65535
    }
  }
}
```

**Benefits of using external schema files:**
- **Separation of concerns**: Keep large schemas in separate files for better maintainability
- **Reusability**: Share schemas across multiple templates or questions
- **Version control**: Better diff viewing for schema changes
- **Editor support**: Most editors provide JSON Schema validation and autocompletion

**Note:** Don't forget to add schema files to your `.bakerignore` to prevent them from being copied to the output:

```
# .bakerignore
*.schema.json
```

#### Result

When prompted for JSON input, the user is given multiple options:

1. Open in external text editor
2. Enter multi-line input in console

```
Configure your database settings - Choose input method:
> Use text editor
  Enter inline
```

JSON data can be accessed in templates like any other nested structure:

```
Connection string: {{ database_config.engine }}://{{ database_config.host }}:{{ database_config.port }}
```

### YAML Complex Type

The YAML type works similarly to the JSON type but uses YAML syntax, which is more readable and less verbose.

#### Example

```yaml
schemaVersion: v1

questions:
  environments:
    type: yaml
    help: Configure your deployment environments
    default:
      development:
        url: http://localhost:8000
        debug: true
      staging:
        url: https://staging.example.com
        debug: true
      production:
        url: https://example.com
        debug: false
```

- **`type`**: Must be `yaml`.
- **`help`**: Should be a string, optionally containing a `minijinja` template.
- **`schema`**: Optional JSON Schema for validation (same format as for JSON type).
- **`schema_file`**: Optional path to an external JSON Schema file (relative to template root). Takes precedence over inline `schema`.
- **`default`**: YAML data, can be provided as a string or native YAML object.

Like the JSON type, YAML questions also support external schema files using `schema_file`. See the [JSON Complex Type](#json-complex-type) section for details on using external schema files.

#### Result

Similar to JSON input, the user is prompted to choose an input method. YAML is particularly useful for configuration data due to its readability:

```
Define your environments:

development:
  url: http://localhost:8000
  debug: true
staging:
  url: https://staging.example.com
  debug: true
production:
  url: https://example.com
  debug: false
```

Template usage:

```
{% for env_name, env_config in environments|items %}
[{{ env_name }}]
URL={{ env_config.url }}
DEBUG={{ env_config.debug }}

{% endfor %}
```

### Validation

Baker supports answer validation using the `validation` attribute. The `condition` attribute uses MiniJinja's expression language to validate user input, while `error_message` provides feedback when validation fails.

#### Required Field Validation

Ensure a field is not empty:

```yaml
schemaVersion: v1

questions:
  age:
    type: str
    help: "Enter your age"
    validation:
      condition: "age"
      error_message: "Value cannot be empty"
```

#### Numeric Value Validation

Check if a numeric value meets certain criteria:

```yaml
schemaVersion: v1

questions:
  age:
    type: str
    help: "Enter your age"
    validation:
      condition: "age|int >= 18"
      error_message: "You must be at least 18 years old. You entered {{age}}."
```

The error message can include template variables to provide context about the invalid input.

#### Pattern Matching with Regular Expressions

Complex validation combining regex pattern matching with numeric validation and detailed error messages:

```yaml
schemaVersion: v1

questions:
  age:
    type: str
    help: Enter your age
    validation:
      condition: "age and (age|regex('[0-9]')) and (age|int >= 18)"
      error_message: >
        {% if not age %}Age is required field
        {% elif not age|regex('[0-9]') %}Age must be numeric
        {% elif not age|int >= 18 %}You must be at least 18 years old. You entered {{age}}
        {% else %}Invalid input
        {% endif %}
```

This example demonstrates:

1. Required field validation using `age`
2. Pattern matching using `regex('[0-9]')` to ensure numeric input
3. Numeric value validation ensuring age is at least 18
4. Conditional error messages that provide specific feedback based on the validation failure

If validation fails, Baker will:

1. Display the appropriate error message
2. Clear the invalid answer
3. Prompt the user to try again

### Conditional questions

The `ask_if` attribute is used to control the display of a question, using [expression language](https://docs.rs/minijinja/latest/minijinja/#expression-usage) from MiniJinja. It enables conditional logic to determine whether a question should be prompted based on user input or other contextual factors. In the following example, the `py_framework` question is only prompted if the user selects `Python` as the programming language in the `language` question:

```yaml
schemaVersion: v1

questions:
  language:
    type: str
    help: What is your programming language?
    default: Rust
    choices:
      - Python
      - Rust
      - Go
      - TypeScript
  py_framework:
    type: str
    help: What is your Python framework?
    choices:
      - Django
      - FastAPI
      - Pyramid
      - Tornado
    ask_if: "language == 'Python'"
```

## Built-in Filters

Baker provides a set of built-in filters and functions to enhance the flexibility of your templates. These are powered by the MiniJinja templating engine and additional custom filters.

### Available Filters

| **Filter Name**        | **Description**                                               |
| ---------------------- | ------------------------------------------------------------- |
| `camel_case`           | Converts a string to camelCase.                               |
| `kebab_case`           | Converts a string to kebab-case.                              |
| `pascal_case`          | Converts a string to PascalCase.                              |
| `screaming_snake_case` | Converts a string to SCREAMING_SNAKE_CASE.                    |
| `snake_case`           | Converts a string to snake_case.                              |
| `table_case`           | Converts a string to table_case (lowercase with underscores). |
| `train_case`           | Converts a string to Train-Case.                              |
| `plural`               | Converts a word to its plural form.                           |
| `singular`             | Converts a word to its singular form.                         |
| `foreign_key`          | Converts a string to a foreign key format (e.g., `user_id`).  |
| `regex`                | Applies a regular expression to transform a string.           |

### Usage Examples

#### 1. Camel Case Filter

```yaml
{{ "hello world" | camel_case }}
// Output: "helloWorld"
```

#### 2. Kebab Case Filter

```yaml
{{ "hello world" | kebab_case }}
// Output: "hello-world"
```

#### 3. Pascal Case Filter

```yaml
{{ "hello world" | pascal_case }}
// Output: "HelloWorld"
```

#### 4. Screaming Snake Case Filter

```yaml
{{ "hello world" | screaming_snake_case }}
// Output: "HELLO_WORLD"
```

#### 5. Snake Case Filter

```yaml
{{ "hello world" | snake_case }}
// Output: "hello_world"
```

#### 6. Table Case Filter

```yaml
{{ "Hello World" | table_case }}
// Output: "hello_world"
```

#### 7. Train Case Filter

```yaml
{{ "hello world" | train_case }}
// Output: "Hello-World"
```

#### 8. Plural Filter

```yaml
{{ "car" | plural }}
// Output: "cars"
```

#### 9. Singular Filter

```yaml
{{ "cars" | singular }}
// Output: "car"
```

#### 10. Foreign Key Filter

```yaml
{{ "User" | foreign_key }}
// Output: "user_id"
```

#### 11. Regex Filter

```yaml
{{ "hello world" | regex: "world", "Rust" }}
// Output: "hello Rust"
```

## Comparing Baker to other project generators

| Feature                                           | Baker                                                                                | Kickstart     | cargo-generate         | Copier                                    | Cookiecutter              | Yeoman                       |
|---------------------------------------------------|--------------------------------------------------------------------------------------|---------------|------------------------|-------------------------------------------|---------------------------|------------------------------|
| 🟢 **Structured JSON/YAML input**                 | ✅ Native support with validation and schema                                          | ❌             | ❌                      | ⚠️ Limited                                | ❌                         | ⚠️ Custom logic required     |
| 🟢 **JSON Schema validation**                     | ✅ Enforce data validity with standard JSON Schema                                    | ❌             | ❌                      | ❌                                         | ❌                         | ⚠️ Custom logic required     |
| 🟢 **Complex data editing modes**                 | ✅ Editor/Console/File input for structured data                                      | ❌             | ❌                      | ❌                                         | ❌                         | ❌                            |
| 🟢 **In-template debug() support**                | ✅ Use `{{ debug() }}` to inspect context                                             | ❌             | ❌                      | ❌                                         | ❌                         | ⚠️ Only via console.log      |
| 🟢 **Structured hook communication**              | ✅ pre/post hooks exchange structured JSON via stdin/stdout                           | ❌             | ❌                      | ❌                                         | ❌                         | ❌                            |
| 🟢 **Safe hook execution**                        | ✅ Warns before executing hooks                                                       | ❌             | ❌                      | ❌                                         | ❌                         | ⚠️ Depends on generator      |
| 🟢 **Schema versioning for config**               | ✅ Schema version ensures backward compatibility across Baker versions                | ✅             | ❌                      | ❌                                         | ❌                         | ❌                            |
| 🟢 **YAML & JSON config support**                 | ✅ Supports `yaml` **and** `json` configurations                                      | ❌ Only TOML   | ❌ Only TOML            | ❌ Only YAML                               | ❌ Only JSON               | ❌ In JS code                 |
| 🟢 **Platform-specific hooks**                    | ✅ Use `{{platform.family}}/pre` etc. for OS-aware logic                              | ❌             | ⚠️ Limited via Rhai    | ❌                                         | ❌                         | ⚠️ Custom logic required     |
| 🟢 **CI/CD-friendly answers piping**              | ✅ `--answers=-` or echo JSON into CLI                                                | ❌             | ⚠️ Partial             | ✅ Via pre-filled YAML                     | ⚠️ `--no-input` only      | ❌ Manual scripting           |
| 🟢 **Lightweight & Fast**                         | ✅ Rust binary, no runtime dependencies                                               | ✅ Rust binary | ✅ Rust binary          | ❌ Requires Python                         | ❌ Requires Python         | ❌ Requires Node.js           |
| 🟢 **Simple CLI Interface**                       | ✅ `baker <template> <output>` + `--answers`, `--skip-confirms`                       | ✅ Simple      | ❌ Requires Cargo usage | ❌ More verbose                            | ✅ Simple                  | ❌ Requires generator install |
| 🟢 **Language-agnostic hooks**                    | ✅ Hooks can be in _any_ language (Bash, Python, etc.)                                | ✅ Yes         | ⚠️ Only Rhai scripting | ✅ Yes                                     | ✅ Yes                     | ❌ Only JS                    |
| 🟢 **Templated file/dir names**                   | ✅ Full MiniJinja templating in names & conditions                                    | ✅ Yes         | ✅ Yes                  | ✅ Yes                                     | ✅ Yes                     | ✅ Via JS logic               |
| 🟢 **Templated prompts & defaults**               | ✅ Dynamic defaults using MiniJinja, conditional via `ask_if`                         | ✅ Yes         | ⚠️ Limited             | ✅ Full Jinja                              | ❌ Static only             | ✅ Full control in JS         |
| 🟢 **Glob-based ignore file**                     | ✅ `.bakerignore` with advanced Globset syntax                                        | ✅ Yes         | ✅ Yes                  | ✅ `_exclude`                              | ⚠️ `_copy_without_render` | ❌ Manual filter in code      |
| 🟢 **Cross-platform binaries**                    | ✅ Precompiled for Linux, macOS, Windows                                              | ✅ Yes         | ✅ Yes                  | ✅ Yes                                     | ✅ Yes                     | ✅ Yes                        |
| 🟢 **Language-agnostic scaffolding**              | ✅ Works with any language / stack                                                    | ✅ Yes         | ❌ Rust-focused         | ✅ Yes                                     | ✅ Yes                     | ⚠️ JS-centric                |
| 🟢 **Answers accessible in later questions**      | ✅ All previous answers available via MiniJinja in `default`, `help`, `ask_if`        | ⚠️ Limited    | ⚠️ Partial (via Rhai)  | ✅ Yes (Jinja context)                     | ❌                         | ✅ Full control in JS         |
| 🟢 **Templated engine**                           | ✅ Fast, safe, embedded Jinja2-like templating in Rust                                | Tera          | Liquid                 | Jinja2                                    | Jinja2                    | EJS                          |
| 🟢 **Looping of files (template filename loops)** | ✅ Generate multiple files from a single template using Jinja2 for-loops in filenames | ❌             | ❌                      | ⚠️ Limited (Jinja2 loops in content only) | ❌                         | ⚠️ Custom logic required     |

### ℹ️ Disclaimer

This comparison was made based on available documentation. If you notice any **inaccuracies or outdated information**, please [create an issue](https://github.com/aliev/baker/issues) — I'll be happy to update the table accordingly.

## Community Templates

See [here](https://github.com/topics/baker-template) for a list of community maintained templates built with baker.
