/// Visual themes for interactive terminal prompts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptTheme {
    /// Uses dialoguer's default colorful look.
    Classic,
    /// Uses Baker-branded colors and prefixes.
    #[default]
    Fancy,
}
