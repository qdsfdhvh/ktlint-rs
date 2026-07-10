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
}
