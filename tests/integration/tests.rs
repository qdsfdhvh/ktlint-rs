#[cfg(test)]
mod integration_tests {
    use std::process::Command;
    use std::path::PathBuf;

    fn ktlint_bin() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/debug/ktlint")
    }

    fn fixtures_dir(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn ensure_built() {
        // Build debug binary if not exists
        let bin = ktlint_bin();
        if !bin.exists() {
            let status = Command::new("cargo")
                .args(["build"])
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .status()
                .expect("Failed to build");
            assert!(status.success());
        }
    }

    // ── EditorConfig tests ──

    #[test]
    fn editorconfig_2space_indent_works() {
        ensure_built();
        let output = Command::new(ktlint_bin())
            .arg(fixtures_dir("editorconfig_2space"))
            .output()
            .expect("ktlint failed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        // 2-space indentation should NOT trigger indent violations
        assert!(!stdout.contains("standard:indent"),
            "2-space indent should not cause violations:\n{}", stderr);
    }

    #[test]
    fn editorconfig_android_style_disables_rules() {
        ensure_built();
        let output = Command::new(ktlint_bin())
            .arg(fixtures_dir("editorconfig_android"))
            .output()
            .expect("ktlint failed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        // android_studio disables final-newline
        assert!(!stdout.contains("standard:final-newline"),
            "android_studio should disable final-newline");
    }

    #[test]
    fn editorconfig_rule_disabling_works() {
        ensure_built();
        let output = Command::new(ktlint_bin())
            .arg(fixtures_dir("editorconfig_rules"))
            .output()
            .expect("ktlint failed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        // These rules are disabled in .editorconfig
        assert!(!stdout.contains("standard:no-wildcard-imports"), "wildcard-imports should be disabled");
        assert!(!stdout.contains("standard:curly-spacing"), "curly-spacing should be disabled");
        // But other rules still work
        assert!(stdout.contains("standard:op-spacing") || stdout.contains("standard:colon"),
            "Other rules should still work");
    }

    #[test]
    fn editorconfig_tab_indent_detected() {
        ensure_built();
        let output = Command::new(ktlint_bin())
            .arg(fixtures_dir("editorconfig_tab"))
            .output()
            .expect("ktlint failed");
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Tab indent should be loaded from .editorconfig
        assert!(!stderr.contains("Failed") && !stderr.contains("error"),
            "Tab indent .editorconfig should load without errors: {}", stderr);
    }

    // ── CLI tests ──

    #[test]
    fn cli_json_reporter_works() {
        ensure_built();
        let output = Command::new(ktlint_bin())
            .args(["--reporter", "json"])
            .arg(fixtures_dir("editorconfig_rules"))
            .output()
            .expect("ktlint failed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("["), "JSON output should start with [");
        assert!(stdout.contains("rule"), "JSON should contain rule field");
    }

    #[test]
    fn cli_format_modifies_file() {
        ensure_built();
        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/editorconfig_rules/src/Code.kt");
        let original = std::fs::read_to_string(&src).unwrap();
        
        // Run format
        Command::new(ktlint_bin())
            .args(["--format"])
            .arg(fixtures_dir("editorconfig_rules"))
            .output()
            .expect("ktlint format failed");

        let formatted = std::fs::read_to_string(&src).unwrap();
        // Restore original
        std::fs::write(&src, original.as_bytes()).unwrap();
        
        assert_ne!(original, formatted, "Format should modify the file");
    }

    #[test]
    fn cli_version_flag() {
        ensure_built();
        let output = Command::new(ktlint_bin())
            .arg("--version")
            .output()
            .expect("ktlint failed");
        assert!(output.status.success(), "--version should succeed");
    }

    // ── Config tests ──

    #[test]
    fn config_code_style_profiles() {
        let config = crate::config::KtlintConfig::default();
        assert!(config.is_rule_enabled("standard:curly-spacing"));
        
        // android_studio disables certain rules
        let mut android = crate::config::KtlintConfig::default();
        android.code_style = crate::config::CodeStyle::AndroidStudio;
        assert!(!android.is_rule_enabled("standard:final-newline"));
        assert!(!android.is_rule_enabled("standard:no-wildcard-imports"));
        assert!(!android.is_rule_enabled("standard:import-ordering"));
    }

    #[test]
    fn config_per_rule_enable_disable() {
        let mut config = crate::config::KtlintConfig::default();
        config.rules.insert(
            "standard:curly-spacing".to_string(),
            crate::config::RuleConfig { enabled: false, properties: Default::default() },
        );
        assert!(!config.is_rule_enabled("standard:curly-spacing"));
        assert!(config.is_rule_enabled("standard:op-spacing"));
    }

    #[test]
    fn config_indent_string_4_space() {
        let config = crate::config::KtlintConfig::default();
        assert_eq!(config.indent_string(), "    ");
    }

    #[test]
    fn config_indent_string_tab() {
        let mut config = crate::config::KtlintConfig::default();
        config.indent_style = crate::config::IndentStyle::Tab;
        assert_eq!(config.indent_string(), "\t");
    }

    // ── Real-world project smoke tests ──
    //
    // androidx and compose-samples are multi-project collections.
    // We traverse first-level subdirectories (each is a self-contained
    // Gradle project) rather than checking the root directly.

    /// Returns first-level subdirs of `parent` that contain at least one .kt file.
    fn kt_subdirs(parent: &PathBuf) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    // Skip non-project dirs
                    if name.starts_with('.') || name == "docs" || name == "gradle"
                        || name == "buildSrc" || name == "scripts" || name == "readme"
                    {
                        continue;
                    }
                    // Quick check: does this dir have .kt files?
                    let has_kt = std::fs::read_dir(&path)
                        .map(|mut rd| rd.any(|e| {
                            e.map(|f| {
                                f.file_name()
                                    .to_str()
                                    .map(|s| s.ends_with(".kt"))
                                    .unwrap_or(false)
                            })
                            .unwrap_or(false)
                        }))
                        .unwrap_or(false);
                    if has_kt {
                        dirs.push(path);
                    } else {
                        // Deeper check — walk one level
                        if let Ok(sub) = std::fs::read_dir(&path) {
                            let found = sub.flatten().any(|e| {
                                let p = e.path();
                                p.is_dir()
                                    && std::fs::read_dir(&p)
                                        .map(|mut rd| {
                                            rd.any(|f| {
                                                f.map(|x| {
                                                    x.file_name()
                                                        .to_str()
                                                        .map(|s| s.ends_with(".kt"))
                                                        .unwrap_or(false)
                                                })
                                                .unwrap_or(false)
                                            })
                                        })
                                        .unwrap_or(false)
                            });
                            if found {
                                dirs.push(path);
                            }
                        }
                    }
                }
            }
        }
        dirs.sort();
        dirs
    }

    // ── compose-samples: all 6 sample projects ──

    #[test]
    fn real_compose_samples_smoke() {
        ensure_built();
        let base = fixtures_dir("compose-samples");
        let dirs = kt_subdirs(&base);
        assert!(!dirs.is_empty(), "Expected compose-samples subdirs with Kotlin files");

        for dir in &dirs {
            let name = dir.file_name().unwrap().to_str().unwrap();
            let output = Command::new(ktlint_bin())
                .arg(dir)
                .output()
                .expect(&format!("ktlint failed to run on compose-samples/{}", name));

            let stderr = String::from_utf8_lossy(&output.stderr);
            // ktlint should not crash or report internal errors
            assert!(
                !stderr.contains("thread 'main' panicked")
                    && !stderr.contains("Error: Failed")
                    && !stderr.contains("fatal runtime error"),
                "compose-samples/{}: ktlint crashed:\n{}",
                name, stderr
            );

            eprintln!(
                "compose-samples/{}: exit={}, stdout_lines={}, stderr_lines={}",
                name,
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stdout).lines().count(),
                stderr.lines().count()
            );
        }
    }

    // ── androidx: selected smaller modules (skip compose/ 6.6K files — too slow for CI) ──

    /// androidx modules suitable for quick smoke testing (<1000 .kt files each)
    const ANDROIDX_SMOKE_DIRS: &[&str] = &[
        "activity",
        "annotation",
        "autofill",
        "biometric",
        "browser",
        "collection",
        "concurrent",
        "datastore",
        "documentfile",
        "drawerlayout",
        "emoji",
        "fragment",
        "graphics",
        "gridlayout",
        "loader",
        "palette",
        "preference",
        "print",
        "savedstate",
        "slidingpanelayout",
        "startup",
        "swiperefreshlayout",
        "transition",
        "vectordrawable",
        "viewpager",
        "viewpager2",
    ];

    #[test]
    fn real_androidx_smoke() {
        ensure_built();
        let base = fixtures_dir("androidx");

        for dir_name in ANDROIDX_SMOKE_DIRS {
            let dir = base.join(dir_name);
            if !dir.exists() {
                eprintln!("androidx/{}: directory not found, skipping", dir_name);
                continue;
            }

            let output = Command::new(ktlint_bin())
                .arg(&dir)
                .output()
                .expect(&format!("ktlint failed to run on androidx/{}", dir_name));

            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                !stderr.contains("thread 'main' panicked")
                    && !stderr.contains("Error: Failed")
                    && !stderr.contains("fatal runtime error"),
                "androidx/{}: ktlint crashed:\n{}",
                dir_name, stderr
            );

            eprintln!(
                "androidx/{}: exit={}, stdout_lines={}, stderr_lines={}",
                dir_name,
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stdout).lines().count(),
                stderr.lines().count()
            );
        }
    }
}
