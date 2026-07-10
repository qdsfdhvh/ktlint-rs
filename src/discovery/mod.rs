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
        let mut files: Vec<PathBuf> = vec![];

        // Step 1: Direct file paths (absolute or relative to cwd)
        for pattern in &self.cli.patterns {
            let path = std::path::Path::new(pattern);
            if path.is_file() {
                match path.extension().and_then(|e| e.to_str()) {
                    Some("kt") | Some("kts") => {
                        files.push(path.to_path_buf());
                    }
                    _ => {}
                }
            } else if path.is_dir() {
                // Directory: walk it
                let mut builder = WalkBuilder::new(path);
                builder.git_ignore(true).hidden(false).follow_links(false);
                for entry in builder.build() {
                    let entry = entry?;
                    let entry_path = entry.path();
                    if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                        continue;
                    }
                    match entry_path.extension().and_then(|e| e.to_str()) {
                        Some("kt") | Some("kts") => {
                            files.push(entry_path.to_path_buf());
                        }
                        _ => {}
                    }
                }
            }
        }

        // Step 2: Walk project root if no patterns or for project tree discovery
        if self.cli.patterns.is_empty() || !files.is_empty() {
            // Already collected explicit files; still walk project for remaining
            if !self.cli.patterns.is_empty() && !files.is_empty() {
                return Ok(files); // explicit files only
            }
        }

        // Walk project root
        let mut builder = WalkBuilder::new(root);
        builder
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .add_custom_ignore_filename(".ktlintignore")
            .hidden(false)
            .follow_links(false);

        for entry in builder.build() {
            let entry = entry?;
            let path = entry.path();
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            match path.extension().and_then(|e| e.to_str()) {
                Some("kt") | Some("kts") => {
                    if self.matches_patterns(path) {
                        files.push(path.to_path_buf());
                    }
                }
                _ => {}
            }
        }

        Ok(files)
    }

    /// Check if a file path matches the CLI patterns (if any).
    fn matches_patterns(&self, path: &std::path::Path) -> bool {
        if self.cli.patterns.is_empty() {
            return true;
        }
        let path_str = path.to_string_lossy();
        self.cli
            .patterns
            .iter()
            .any(|p| path_str.contains(p.as_str()))
    }
}
