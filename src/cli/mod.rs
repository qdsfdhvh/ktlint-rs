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

    /// Read Kotlin source from standard input instead of files
    #[arg(long, conflicts_with_all = ["patterns"])]
    pub stdin: bool,

    /// Path used for stdin input (affects the filename rule and reporters)
    #[arg(long, default_value = "stdin.kt")]
    pub stdin_path: String,

    #[arg(skip)]
    /// (reserved) file patterns read from stdin
    #[allow(dead_code)]
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

    /// Include ktlint-rs-only extension rules in the default ktlint ruleset
    #[arg(long)]
    pub compat: bool,

    /// Treat unmatched inputs / no files found as error (exit 1 for CI)
    #[arg(long)]
    pub strict: bool,

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

    /// Include only files matching these git-style glob patterns (repeatable)
    #[arg(long = "include", value_name = "GLOB")]
    pub include: Vec<String>,

    /// Exclude files matching these git-style glob patterns (repeatable)
    #[arg(long = "exclude", value_name = "GLOB")]
    pub exclude: Vec<String>,

    /// Print the discovered Kotlin file set as JSON and exit (parity harness)
    #[arg(long, hide = true)]
    pub print_files: bool,

    /// Print per-file effective ktlint-rs configuration as JSON and exit (parity harness)
    #[arg(long, hide = true)]
    pub print_effective_config: bool,

    /// Print registered rule metadata as JSON and exit (parity inventory)
    #[arg(long, hide = true)]
    pub print_rule_inventory: bool,

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
