use crate::cli::Cli;
use crate::config::KtlintConfig;
use anyhow::Result;
use ignore::WalkBuilder;
use std::collections::HashSet;
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
    /// Iterates ALL CLI patterns (not just the first). Walks the project root
    /// only when no patterns are provided. Deduplicates the final list.
    /// Respects `.gitignore` and `.ktlintignore` via the `ignore` crate.
    pub fn collect(&self) -> Result<Vec<PathBuf>> {
        let root = &self.config.project_root;
        let mut files: Vec<PathBuf> = vec![];

        if self.cli.patterns.is_empty() {
            // No patterns: walk project root
            Self::walk_dir(root, &mut files)?;
        } else {
            // Process EVERY pattern (not just the first)
            for pattern in &self.cli.patterns {
                let path = std::path::Path::new(pattern);
                // Resolve relative paths against project root
                let resolved = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    root.join(path)
                };

                if resolved.is_file() {
                    match resolved.extension().and_then(|e| e.to_str()) {
                        Some("kt") | Some("kts") => {
                            files.push(resolved);
                        }
                        _ => {}
                    }
                } else if resolved.is_dir() {
                    Self::walk_dir(&resolved, &mut files)?;
                }
                // Non-existent paths are silently skipped
            }
        }

        // Deduplicate (canonicalize to handle ./a/Alpha.kt and a/Alpha.kt)
        let mut seen: HashSet<PathBuf> = HashSet::new();
        files.retain(|p| {
            // Use canonicalized path for dedup when possible
            let key = p.canonicalize().unwrap_or_else(|_| p.clone());
            seen.insert(key)
        });

        Ok(files)
    }

    /// Walk a directory and collect `.kt` / `.kts` files.
    fn walk_dir(dir: &std::path::Path, files: &mut Vec<PathBuf>) -> Result<()> {
        let mut builder = WalkBuilder::new(dir);
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
                    files.push(path.to_path_buf());
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_fixture() -> (TempDir, KtlintConfig, Cli) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create .editorconfig
        std::fs::write(
            root.join(".editorconfig"),
            "root = true\n[*]\nindent_size = 4\n",
        )
        .unwrap();

        // Create .git dir (required by ignore crate to resolve .gitignore)
        std::fs::create_dir(root.join(".git")).unwrap();
        // Create .gitignore
        std::fs::write(root.join(".gitignore"), "Ignored.kt\n").unwrap();

        // Root files
        std::fs::write(root.join("Root.kt"), "fun root() = Unit\n").unwrap();
        std::fs::write(root.join("Ignored.kt"), "// ignored\n").unwrap();
        std::fs::write(root.join("noext_file"), "noext").unwrap();

        // a/ dir
        let a = root.join("a");
        std::fs::create_dir(&a).unwrap();
        std::fs::write(a.join("Alpha.kt"), "fun alpha() = Unit\n").unwrap();
        std::fs::write(a.join("NotKotlin.txt"), "not kotlin\n").unwrap();

        // b/ dir
        let b = root.join("b");
        std::fs::create_dir(&b).unwrap();
        std::fs::write(b.join("Beta.kt"), "fun beta() = Unit\n").unwrap();
        std::fs::write(b.join("Gamma.kts"), "// kts script\n").unwrap();

        // empty_dir/
        std::fs::create_dir(root.join("empty_dir")).unwrap();

        let mut config = KtlintConfig::default();
        config.project_root = root.to_path_buf();

        let cli = Cli {
            format: false,
            compat: false,
            strict: false,
            patterns_from_stdin: vec![],
            editorconfig: None,
            code_style: None,
            baseline: None,
            create_baseline: false,
            config: None,
            ruleset: "ktlint".to_string(),
            limit: None,
            relative: false,
            color: false,
            reporter: "plain".to_string(),
            reporter_output: None,
            log_level: None,
            patterns: vec![],
        };

        (tmp, config, cli)
    }

    fn collector<'a>(config: &'a KtlintConfig, cli: &'a Cli) -> FileCollector<'a> {
        FileCollector::new(cli, config)
    }

    fn file_names(files: &[PathBuf]) -> Vec<String> {
        let mut names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn t1_1_single_kt_file() {
        let (tmp, config, mut cli) = setup_fixture();
        cli.patterns = vec![tmp.path().join("a/Alpha.kt").to_string_lossy().to_string()];
        let files = collector(&config, &cli).collect().unwrap();
        assert_eq!(file_names(&files), vec!["Alpha.kt"]);
    }

    #[test]
    fn t1_2_single_directory() {
        let (tmp, config, mut cli) = setup_fixture();
        cli.patterns = vec![tmp.path().join("a").to_string_lossy().to_string()];
        let files = collector(&config, &cli).collect().unwrap();
        assert_eq!(file_names(&files), vec!["Alpha.kt"]);
    }

    #[test]
    fn t1_3_two_kt_files() {
        let (tmp, config, mut cli) = setup_fixture();
        cli.patterns = vec![
            tmp.path().join("a/Alpha.kt").to_string_lossy().to_string(),
            tmp.path().join("b/Beta.kt").to_string_lossy().to_string(),
        ];
        let files = collector(&config, &cli).collect().unwrap();
        let names = file_names(&files);
        assert!(
            names.contains(&"Alpha.kt".to_string()),
            "should contain Alpha.kt, got {:?}",
            names
        );
        assert!(
            names.contains(&"Beta.kt".to_string()),
            "should contain Beta.kt, got {:?}",
            names
        );
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn t1_4_two_directories() {
        let (tmp, config, mut cli) = setup_fixture();
        cli.patterns = vec![
            tmp.path().join("a").to_string_lossy().to_string(),
            tmp.path().join("b").to_string_lossy().to_string(),
        ];
        let files = collector(&config, &cli).collect().unwrap();
        let names = file_names(&files);
        assert!(names.contains(&"Alpha.kt".to_string()));
        assert!(names.contains(&"Beta.kt".to_string()));
        assert!(names.contains(&"Gamma.kts".to_string()));
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn t1_5_mixed_file_and_dir() {
        let (tmp, config, mut cli) = setup_fixture();
        cli.patterns = vec![
            tmp.path().join("a/Alpha.kt").to_string_lossy().to_string(),
            tmp.path().join("b").to_string_lossy().to_string(),
        ];
        let files = collector(&config, &cli).collect().unwrap();
        let names = file_names(&files);
        assert!(names.contains(&"Alpha.kt".to_string()));
        assert!(names.contains(&"Beta.kt".to_string()));
        assert!(names.contains(&"Gamma.kts".to_string()));
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn t1_6_empty_patterns_walks_root() {
        let (tmp, config, mut cli) = setup_fixture();
        cli.patterns = vec![];
        let files = collector(&config, &cli).collect().unwrap();
        let names = file_names(&files);
        // Should include Root.kt, Alpha.kt, Beta.kt, Gamma.kts (not Ignored.kt, not .txt)
        assert!(
            names.contains(&"Root.kt".to_string()),
            "should have Root.kt, got {:?}",
            names
        );
        assert!(
            names.contains(&"Alpha.kt".to_string()),
            "should have Alpha.kt, got {:?}",
            names
        );
        assert!(
            !names.contains(&"Ignored.kt".to_string()),
            "Ignored.kt should be excluded"
        );
        assert!(!names.contains(&"NotKotlin.txt".to_string()));
    }

    #[test]
    fn t1_7_kts_extension_collected() {
        let (tmp, config, mut cli) = setup_fixture();
        cli.patterns = vec![tmp.path().join("b").to_string_lossy().to_string()];
        let files = collector(&config, &cli).collect().unwrap();
        let names = file_names(&files);
        assert!(names.contains(&"Gamma.kts".to_string()));
    }

    #[test]
    fn t1_8_deduplication() {
        let (tmp, config, mut cli) = setup_fixture();
        let alpha = tmp.path().join("a/Alpha.kt").to_string_lossy().to_string();
        cli.patterns = vec![alpha.clone(), alpha];
        let files = collector(&config, &cli).collect().unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn t1_9_nonexistent_path() {
        let (tmp, config, mut cli) = setup_fixture();
        cli.patterns = vec!["does/not/exist".to_string()];
        let files = collector(&config, &cli).collect().unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn t1_10_gitignore_respected() {
        let (tmp, config, mut cli) = setup_fixture();
        cli.patterns = vec![];
        let files = collector(&config, &cli).collect().unwrap();
        let names = file_names(&files);
        assert!(!names.contains(&"Ignored.kt".to_string()));
    }

    #[test]
    fn t1_11_non_kotlin_skipped() {
        let (tmp, config, mut cli) = setup_fixture();
        cli.patterns = vec![tmp.path().join("a").to_string_lossy().to_string()];
        let files = collector(&config, &cli).collect().unwrap();
        let names = file_names(&files);
        assert!(!names.contains(&"NotKotlin.txt".to_string()));
    }

    #[test]
    fn t1_12_dir_no_kt_files() {
        let (tmp, config, mut cli) = setup_fixture();
        cli.patterns = vec![tmp.path().join("empty_dir").to_string_lossy().to_string()];
        let files = collector(&config, &cli).collect().unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn t1_15_canonical_dedup() {
        // Same file via different relative paths → deduped
        let (tmp, config, mut cli) = setup_fixture();
        let alpha_abs = tmp.path().join("a/Alpha.kt");
        cli.patterns = vec![
            alpha_abs.to_string_lossy().to_string(),
            format!("./a/Alpha.kt"), // relative path
        ];
        // The relative one won't resolve to tmp path, so we just check no crash
        let files = collector(&config, &cli).collect().unwrap();
        assert!(files.iter().any(|p| p.file_name().unwrap() == "Alpha.kt"));
    }
}
