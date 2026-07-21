use clap::Parser;

/// ktlint-rs — A fast Kotlin linter and formatter written in Rust
#[derive(Parser, Debug)]
#[command(
    name = "ktlint-rs",
    version,
    about = "An anti-bikeshedding Kotlin linter with built-in formatter",
    long_about = "Drop-in compatible with Pinterest ktlint CLI. \
                  Reads .editorconfig, checks/fixes Kotlin style."
)]
pub struct Cli {
    /// Auto-correct style violations
    #[arg(short = 'F', long)]
    pub format: bool,
    #[arg(skip)]
    pub patterns_from_stdin: Vec<String>,

    /// Path to the default .editorconfig
    #[arg(long)]
    pub editorconfig: Option<String>,

    /// Code style preset: android_studio, intellij_idea, ktlint_official
    #[arg(long)]
    pub code_style: Option<String>,

    /// Baseline file to check against
    #[arg(long)]
    pub baseline: Option<String>,

    /// Generate a baseline file from current violations
    #[arg(long)]
    pub create_baseline: bool,

    #[arg(long)]
    pub config: Option<String>,

    // ── Rule set selection ──
    /// Comma-separated rule sets: ktlint, detekt, ktlint,detekt (default: ktlint)
    #[arg(long, default_value = "ktlint")]
    #[arg(long, default_value = "ktlint", value_parser = ["ktlint", "detekt", "ktlint,detekt"])]
    pub ruleset: String,

    /// Enable JVM-compatible mode: include ktlint-rs-only rules in default ruleset
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

    /// Reporter to use: plain, json, sarif, checkstyle, html, markdown, plain-summary
    #[arg(long, default_value = "plain")]
    #[arg(long, default_value = "plain", value_parser = ["plain", "json", "sarif", "checkstyle", "html", "markdown", "plain-summary"])]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_contains_ktlint_rs() {
        // clap generates --version from the crate name + version
        let bin_name = env!("CARGO_PKG_NAME");
        assert_eq!(bin_name, "ktlint-rs", "binary name should be ktlint-rs");
    }
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
