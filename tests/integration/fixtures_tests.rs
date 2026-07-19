//! Integration tests — full fixture validation.
//! Verifies ktlint-rs runs on all test fixtures without crashes.

#[cfg(test)]
mod fixture_tests {
    use std::path::PathBuf;
    use std::process::Command;

    fn bin() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/ktlint-rs")
    }

    fn ensure_built() {
        // Skip in CI — binary is pre-built by workflow
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn run(ruleset: &str, dir: &str) -> (i32, usize, String) {
        ensure_built();
        let output = Command::new(bin())
            .args(["--ruleset", ruleset])
            .arg(fixture(dir))
            .output()
            .unwrap_or_else(|e| panic!("{} {}: failed: {}", ruleset, dir, e));
        let code = output.status.code().unwrap_or(-1);
        let lines = String::from_utf8_lossy(&output.stdout).lines().count();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        (code, lines, stderr)
    }

    fn assert_no_crash(ruleset: &str, dir: &str) {
        let (code, lines, stderr) = run(ruleset, dir);
        assert!(
            code == 0 || code == 1,
            "{} {}: exit {} (expected 0 or 1)",
            ruleset,
            dir,
            code
        );
        assert!(
            !stderr.contains("panicked") && !stderr.contains("Error: Failed"),
            "{} {}: crashed!\nstderr: {}",
            ruleset,
            dir,
            stderr
        );
        eprintln!(
            "  {} {}: exit={}, violations={} lines",
            ruleset, dir, code, lines
        );
    }

    #[test]
    fn fixture_nowinandroid_ktlint() {
        assert_no_crash("ktlint", "nowinandroid");
    }
    #[test]
    fn fixture_nowinandroid_detekt() {
        assert_no_crash("detekt", "nowinandroid");
    }
    #[test]
    fn fixture_okhttp_ktlint() {
        assert_no_crash("ktlint", "okhttp");
    }
    #[test]
    fn fixture_okhttp_detekt() {
        assert_no_crash("detekt", "okhttp");
    }
    #[test]
    fn fixture_compose_ktlint() {
        assert_no_crash("ktlint", "compose-samples");
    }
    #[test]
    fn fixture_androidx_ktlint() {
        assert_no_crash("ktlint", "androidx");
    }
    #[test]
    fn fixture_editorconfig_2space() {
        assert_no_crash("ktlint", "editorconfig_2space");
    }
    #[test]
    fn fixture_editorconfig_tab() {
        assert_no_crash("ktlint", "editorconfig_tab");
    }
    #[test]
    fn fixture_editorconfig_android() {
        assert_no_crash("ktlint", "editorconfig_android");
    }
    #[test]
    fn fixture_editorconfig_mixed() {
        assert_no_crash("ktlint", "editorconfig_mixed");
    }
    #[test]
    fn fixture_editorconfig_sections() {
        assert_no_crash("ktlint", "editorconfig_sections");
    }
    #[test]
    fn fixture_editorconfig_profile() {
        assert_no_crash("ktlint", "editorconfig_profile");
    }
    #[test]
    fn fixture_editorconfig_rules() {
        assert_no_crash("ktlint", "editorconfig_rules");
    }
}
