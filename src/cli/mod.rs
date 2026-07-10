use clap::Parser;

/// ktlint-rs — A fast Kotlin linter and formatter written in Rust
#[derive(Parser, Debug)]
#[command(
    name = "ktlint",
    version,
    about = "An anti-bikeshedding Kotlin linter with built-in formatter",
    long_about = "Drop-in compatible with Pinterest ktlint CLI. \
                  Reads .editorconfig, checks/fixes Kotlin style."
)]
pub struct Cli {
    /// Auto-correct style violations
    #[arg(short = 'F', long)]
    pub format: bool,

    /// Read additional file patterns from stdin
    #[arg(long)]
    pub patterns_from_stdin: bool,

    /// Path to the default .editorconfig
    #[arg(long)]
    pub editorconfig: Option<String>,

    /// Code style preset: android_studio, intellij_idea, ktlint_official
    #[arg(long)]
    pub code_style: Option<String>,

    /// Baseline file to check against
    #[arg(long)]
    pub baseline: Option<String>,

    /// JVM ktlint compatibility mode (disables ktlint-rs-only rules)
    #[arg(long)]
    pub compat: bool,

    /// Maximum number of errors to show
    #[arg(long)]
    pub limit: Option<usize>,

    /// Print file paths relative to working directory
    #[arg(long)]
    pub relative: bool,

    /// Colorize output
    #[arg(long)]
    pub color: bool,

    /// Reporter to use: plain, json, sarif, checkstyle, html, plain-summary
    #[arg(long, default_value = "plain")]
    pub reporter: String,

    /// Reporter output file
    #[arg(long)]
    pub reporter_output: Option<String>,

    /// Log level
    #[arg(short = 'l', long)]
    pub log_level: Option<String>,

    /// File / directory patterns to check
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub patterns: Vec<String>,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
