use crate::cli::Cli;
use crate::config::KtlintConfig;
use anyhow::Result;
use ignore::WalkBuilder;
use std::path::PathBuf;

/// Discovers Kotlin files to lint, respecting .gitignore and file patterns.
pub struct FileCollector<'a> {
    cli: &'a Cli,
    config: &'a KtlintConfig,
}

impl<'a> FileCollector<'a> {
    pub fn new(cli: &'a Cli, config: &'a KtlintConfig) -> Self {
        Self { cli, config }
    }

    /// Collect Kotlin files (`.kt`, `.kts`) from the project.
    ///
    /// If `--patterns-from-stdin` is set, merges stdin patterns with CLI args.
    /// Respects `.gitignore` and `.ktlintignore` via the `ignore` crate.
    pub fn collect(&self) -> Result<Vec<PathBuf>> {
        let root = &self.config.project_root;
        let mut builder = WalkBuilder::new(root);
        builder
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .add_custom_ignore_filename(".ktlintignore")
            .hidden(false) // ktlint processes .kt files in hidden dirs too
            .follow_links(false);

        let mut files: Vec<PathBuf> = vec![];
        for entry in builder.build() {
            let entry = entry?;
            let path = entry.path();
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            match path.extension().and_then(|e| e.to_str()) {
                Some("kt") | Some("kts") => {
                    // Apply user-supplied glob patterns as filters
                    if self.matches_patterns(path) {
                        files.push(path.to_path_buf());
                    }
                }
                _ => {}
            }
        }

        Ok(files)
    }

    fn matches_patterns(&self, path: &std::path::Path) -> bool {
        if self.cli.patterns.is_empty() {
            return true; // no patterns = all Kotlin files
        }
        let path_str = path.to_string_lossy();
        self.cli.patterns.iter().any(|p| {
            // Simplified: exact prefix matching; use glob crate for real patterns
            path_str.contains(p) || path_str.starts_with(p.trim_end_matches('/'))
        })
    }
}
