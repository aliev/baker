//! Constants used throughout the Baker application

/// Configuration file names in order of preference
pub const CONFIG_FILENAMES: &[&str] = &["baker.json", "baker.yaml", "baker.yml"];

/// Default template file suffix
pub const DEFAULT_TEMPLATE_SUFFIX: &str = ".baker.j2";

/// Default template file loop separator
pub const DEFAULT_LOOP_SEPARATOR: &str = "---";

/// Default template file loop content separator
pub const DEFAULT_LOOP_CONTENT_SEPARATOR: &str = "***+++***";

/// Default pre-hook filename
pub const DEFAULT_PRE_HOOK: &str = "pre";

/// Default post-hook filename
pub const DEFAULT_POST_HOOK: &str = "post";

/// Ignore file name
pub const IGNORE_FILE: &str = ".bakerignore";

/// STDIN indicator for CLI arguments
pub const STDIN_INDICATOR: &str = "-";

/// JSON Schema validation messages
pub mod validation {
    pub const INVALID_ANSWER: &str = "Invalid answer";
    pub const PASSWORDS_MISMATCH: &str = "Passwords do not match";
    pub const DEFAULT_CONDITION: &str = "true";
}

/// Exit codes
pub mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const FAILURE: i32 = 1;
}

/// Verbosity levels
pub mod verbosity {
    pub const OFF: u8 = 0;
    pub const INFO: u8 = 1;
    pub const DEBUG: u8 = 2;
    pub const TRACE: u8 = 3;
}
