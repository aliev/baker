# Changelog

All notable changes to this project will be documented in this file.

## [0.12.0] - 2025-10-06

### 🚀 Features

- Add hook runner configuration fields
- Propagate hook runners to execution plan
- Execute hooks via configured runners

### 🐛 Bug Fixes

- Allow templated hook runner tokens
- Tolerate non-UTF8 hook output streams

### 💼 Other

- Emit notice when hook stdout contains non-UTF8 data

### 🚜 Refactor

- Extract cli args and add architecture doc
- Stage runner workflow
- Centralize generation context
- Split file operations helpers
- Streamline template loop handling
- Reorganize prompt module
- Streamline question rendering
- Unify dry-run logging

### 📚 Documentation

- Align hook runner examples with hooks directory convention

### 🧪 Testing

- Rename generic it_works cases
- Add coverage for prompt and runner helpers
- Cover hook runners and update usage
- Add hook runner integration fixtures
- Fix Windows hook runner scripts to read stdin
- Align expected Windows README newline

### ⚙️ Miscellaneous Tasks

- Update readme (#72)

## [0.11.0] - 2025-08-09

### 🚀 Features

- Log ignore patterns and continue loop on TemplateOperation::Ignore
- *(cli)* Add multi-level verbosity support
- *(cli)* Add dry-run support for template processing (#63)
- Support loop in templates (#65)

### 🐛 Bug Fixes

- Removed tests from mod.rs
- Test_yaml_complex_type
- Replace pipe symbol in template filenames for Windows compatibility
- Replace pipe symbols in template filenames for Windows compatibility
- Test_platform_variables should run on macos only
- *(tests)* Change verbose field from boolean to integer in test utilities
- *(cli,prompt)* Correct default log level and choice index handling
- Dependabot dependencies update by removing custom workflow

### 💼 Other

- Reorganize dialoguer module into submodule

### 🚜 Refactor

- Renamed dialoguer to prompt
- Decuple dialoguer from the interface
- Removed legacy dialoguer code
- Removed legacy tests
- Parse methods were moved to parser.rs and covered with tests
- Restructure loader module and simplify template engine API (#51)
- *(cli)* Modularize CLI into focused sub-modules
- *(cli)* Improve module encapsulation and remove ioutils dependencies
- *(config)* Modularize configuration system into separate modules
- Consolidate validation logic into answers module
- *(renderer)* Modularize renderer into separate modules
- *(cli)* Move template import functionality into Runner
- [**breaking**] Modularize CLI components and improve hook handling
- Replace path_to_str function with PathExt trait
- Reduce test duplication (#58)
- Introduce constants module and reorganize hooks

### 📚 Documentation

- Update installation links to use latest release URLs

### 🧪 Testing

- Add comprehensive integration tests for template features
- Increase verbosity level in integration tests

### ⚙️ Miscellaneous Tasks

- Removed dialoguer/utils.ts
- Added adapter.rs tests
- *(ci)* Add GitHub automation and dependency management
- Remove unnecessary GitHub pull request template
- *(dependabot)* Remove deprecated reviewers field
- Remove duplicate tests from build (#54)
- Fix clippy errors
- Increase test coverage (#64)

## [0.10.0] - 2025-06-28

### 🚀 Features

- Add support for loop controls in templates (#33)
- Add changelog automated update with git-cliff (#37)
- Add code coverage (#39)
- Add codecov to build (#42)

### 🐛 Bug Fixes

- Merge pre-hook and CLI answers instead of exclusive selection
- Prevent broken pipe error in hook execution on Linux
- Use inline format args to resolve clippy warnings
- Handle hook stdin write errors with proper logging
- Coverage report workflow (#40)

### 📚 Documentation

- Add contribution guidelines
- Update unreleased changelog entries

## [0.9.0] - 2025-06-22

### 🚀 Features

- Add support for import of macros (#28)
- Enhances error reporting and debugging

### 🐛 Bug Fixes

- Fmt and clippy errors

### 📚 Documentation

- Improve README structure

### ⚙️ Miscellaneous Tasks

- Add test coverage for import directory
- Bump version to 0.9.0

## [0.8.1] - 2025-06-18

### 🐛 Bug Fixes

- The issue when passing the git repository as a template source
- Fmt errors
- Clippy warnings
- Commented unstable features of rustfmt

### ⚙️ Miscellaneous Tasks

- Bump version to 0.8.1

## [0.8.0] - 2025-06-08

### 🚀 Features

- Added answers validation support
- Added proper validation and refactoring
- Improve parsing error handling for user input

### 🐛 Bug Fixes

- Updated table of contents
- Added new validation attribute
- Prevent infinite validation loop for conditional questions
- Improve non-interactive mode behavior and documentation
- Update GitHub Actions runners from ubuntu-20.04 to ubuntu-latest
- The demo template. added README.md to .bakerignore
- Trying to fix the release pipeline

### ⚙️ Miscellaneous Tasks

- Moved ask_question to dialoguer.rs
- Updated documentation in README.md
- Upd version to 0.8.0
- Roll back demo example
- Created a new edit_with_external_editor function

## [0.7.0] - 2025-04-06

### 🚀 Features

- Added json and yaml question types support

### 🐛 Bug Fixes

- Removed mention of file path support from README
- Clippy issues
- Clippy issues
- Formatting issues
- Replaced function .items() with items in readme
- Trying to fix linux-musl pipeline
- Trying to fix linux-musl pipeline
- Using latest version of jsonschema
- Removed x86_64-unknown-linux-musl from build

### 💼 Other

- Version to 0.7.0

### 📚 Documentation

- Add table of contents to README
- Add installation instructions to README
- Add comparison table with other project generators
- Updated comparison table

### ⚙️ Miscellaneous Tasks

- Created process_structured_default_value function

## [0.6.0] - 2025-03-29

### 🚀 Features

- Improve hook output handling and version bump to 0.6.0

## [0.5.0] - 2025-03-28

### ⚙️ Miscellaneous Tasks

- Fix cross-compilation dependencies for multiple targets
- Fix cross-compilation dependencies for multiple targets
- Fix cross-compilation dependencies for multiple targets
- Trying to fix an issue with libz-sys on ARM64 Windows
- Temporarily removed aarch64-pc-windows-msvc from build
- Bump 0.5.0

## [0.4.0] - 2025-03-28

### 🚀 Features

- Trying to add cargo dist
- Bump version

### 🐛 Bug Fixes

- Clippy issues
- Formatting
- Handle error when HOME directory is invalid
- Logging imports and add debug logs for hook execution
- Improve JSON parsing from hook output

## [0.3.0] - 2025-03-21

### 🚀 Features

- Added template strings support in hook filenames

## [0.2.0] - 2025-03-21

### 🚀 Features

- Add configurable hook filenames

### 🐛 Bug Fixes

- Properly handle config file errors
- Removed test template directory
- Using | for baker.yaml and baker.yml
- Leftovers
- Upd README
- Upd README
- Tests on windows
- Skip the tests on Windows for now until we get them fixed

### 🚜 Refactor

- Following the Rust best practices

## [0.1.0] - 2025-01-19

### 🚀 Features

- Added project readme
- Update dependencies and improve error handling in various modules

### 🐛 Bug Fixes

- Release pipeline
- Support for YAML configuration files in config loader
- Improve path handling in bakerignore file parsing
- When the entire file name is in the answers

### ⚙️ Miscellaneous Tasks

- Removed unnecessary comment

<!-- generated by git-cliff -->
