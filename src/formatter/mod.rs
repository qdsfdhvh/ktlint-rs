//! Auto-fix engine — applies corrections for auto-fixable violations.
//!
//! Works on a line-by-line basis for spacing rules, and will be
//! extended to CST-tree-aware fixes for structural rules.

use crate::rules::Violation;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

/// Apply auto-fixes to files and write them back.
pub fn auto_fix(files: &[PathBuf], violations: &[Violation]) -> Result<()> {
    // Group violations by file
    let mut by_file: HashMap<&str, Vec<&Violation>> = HashMap::new();
    for v in violations {
        if v.auto_fixable {
            by_file.entry(&v.file).or_default().push(v);
        }
    }

    for (file_path, file_violations) in &by_file {
        let path = std::path::Path::new(file_path);
        let mut lines: Vec<String> = std::fs::read_to_string(path)?
            .lines()
            .map(|l| l.to_string())
            .collect();

        // Apply fixes in reverse line order to preserve line numbers
        let mut sorted: Vec<_> = file_violations.iter().collect();
        sorted.sort_by_key(|v| std::cmp::Reverse(v.line));
        for v in sorted {
            apply_fix(&mut lines, v);
        }

        // Write back
        let fixed: String = lines.join("\n") + "\n";
        std::fs::write(path, fixed)?;
    }

    Ok(())
}

fn apply_fix(lines: &mut Vec<String>, v: &Violation) {
    let idx = v.line.saturating_sub(1);
    if idx >= lines.len() {
        return;
    }

    match v.rule_id.as_str() {
        "standard:no-trailing-spaces" => {
            lines[idx] = lines[idx].trim_end().to_string();
        }
        "standard:final-newline" => {
            // Handled by the join("\n") + "\n" above
        }
        "standard:no-consecutive-blank-lines" => {
            if lines[idx].trim().is_empty() {
                lines.remove(idx);
            }
        }
        _ => {
            log::debug!("No auto-fix implemented for rule: {}", v.rule_id);
        }
    }
}
