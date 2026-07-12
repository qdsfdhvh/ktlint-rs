//! Git hook support — install/uninstall pre-commit hook.

use std::fs;
use std::path::Path;

/// Content of the pre-commit hook script.
const HOOK_SCRIPT: &str = r#"#!/usr/bin/env bash
# ktlint-rs pre-commit hook — lint staged Kotlin files before commit.
set -euo pipefail

# Find the ktlint-rs binary
if command -v ktlint-rs &>/dev/null; then
    KTLINT="ktlint-rs"
elif [[ -f "$(dirname "$0")/../../target/release/ktlint-rs" ]]; then
    KTLINT="$(cd "$(dirname "$0")/../.." && pwd)/target/release/ktlint-rs"
elif [[ -f "$(dirname "$0")/../../target/debug/ktlint-rs" ]]; then
    KTLINT="$(cd "$(dirname "$0")/../.." && pwd)/target/debug/ktlint-rs"
else
    echo "ktlint-rs: not found. Install with: cargo install ktlint-rs" >&2
    exit 1
fi

# Collect staged Kotlin files
FILES=$(git diff --cached --name-only --diff-filter=ACMR | grep -E '\.kt$|\.kts$' || true)
if [[ -z "$FILES" ]]; then
    exit 0
fi

# Run ktlint-rs on staged files
echo "$FILES" | xargs "$KTLINT"
"#;

/// Install a git pre-commit hook.
pub fn install_git_hook(repo_root: &Path) -> anyhow::Result<()> {
    let git_dir = repo_root.join(".git");
    if !git_dir.exists() {
        anyhow::bail!("No .git directory found at {}. Are you in a git repository?", repo_root.display());
    }

    let hooks_dir = git_dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join("pre-commit");

    // Check if a hook already exists
    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;
        if existing.contains("ktlint-rs") {
            eprintln!("ktlint-rs pre-commit hook is already installed at {}", hook_path.display());
            return Ok(());
        }
        // Append to existing hook
        let combined = format!("{}\n# --- ktlint-rs ---\n{}", existing, HOOK_SCRIPT);
        fs::write(&hook_path, combined)?;
        eprintln!("Appended ktlint-rs to existing pre-commit hook at {}", hook_path.display());
    } else {
        fs::write(&hook_path, HOOK_SCRIPT)?;
        eprintln!("Installed ktlint-rs pre-commit hook at {}", hook_path.display());
    }

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}

/// Uninstall the git pre-commit hook.
pub fn uninstall_git_hook(repo_root: &Path) -> anyhow::Result<()> {
    let hook_path = repo_root.join(".git").join("hooks").join("pre-commit");

    if !hook_path.exists() {
        eprintln!("No pre-commit hook found at {}", hook_path.display());
        return Ok(());
    }

    let content = fs::read_to_string(&hook_path)?;

    if content.contains("ktlint-rs") {
        if content.trim() == HOOK_SCRIPT.trim() {
            // This is a pure ktlint-rs hook — just remove it
            fs::remove_file(&hook_path)?;
            eprintln!("Removed ktlint-rs pre-commit hook from {}", hook_path.display());
        } else {
            // Remove only the ktlint-rs portion
            let cleaned = content
                .lines()
                .filter(|line| !line.contains("ktlint-rs") && !line.starts_with("# --- ktlint-rs"))
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(&hook_path, cleaned.trim_end())?;
            fs::write(&hook_path, format!("{}\n", cleaned.trim_end()))?;
            eprintln!("Removed ktlint-rs portion from pre-commit hook at {}", hook_path.display());
        }
    } else {
        eprintln!("ktlint-rs hook not found in {}", hook_path.display());
    }

    Ok(())
}
