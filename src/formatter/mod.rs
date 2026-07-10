//! Auto-fix engine — applies corrections using CST-aware fixes.
//! Phase 1: text-level patterns (curly, comma, comment, blank-lines).
//! Phase 2: line-based fixes (trailing spaces, blank line removal).

use crate::rules::Violation;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn auto_fix(files: &[PathBuf], violations: &[Violation]) -> Result<()> {
    let mut by_file: HashMap<&str, Vec<&Violation>> = HashMap::new();
    for v in violations {
        if v.auto_fixable {
            by_file.entry(&v.file).or_default().push(v);
        }
    }

    for (file_path, _file_violations) in &by_file {
        let path = std::path::Path::new(file_path);
        let mut text = std::fs::read_to_string(path)?;
        let original = text.clone();

        // Collect unique rule IDs for this file
        let rules: std::collections::HashSet<&str> = _file_violations.iter().map(|v| v.rule_id.as_str()).collect();

        // Apply text-level fixes (each fix is idempotent, so order doesn't matter)
        if rules.contains("standard:curly-spacing") {
            text = fix_curly(&text);
        }
        if rules.contains("standard:comma-spacing") {
            text = fix_comma(&text);
        }
        if rules.contains("standard:comment-spacing") {
            text = fix_comment(&text);
        }
        if rules.contains("standard:no-blank-line-before-rbrace") {
            text = fix_blank_rbrace(&text);
        }
        if rules.contains("standard:multiline-if-else") {
            text = fix_else(&text);
        }

        // Line-based fixes (must be done after text-level to preserve line numbers)
        if rules.contains("standard:no-trailing-spaces") || rules.contains("standard:no-consecutive-blank-lines") {
            text = fix_lines(&text, &rules);
        }

        // Only write if changed
        if text != original {
            std::fs::write(path, text)?;
        }
    }

    Ok(())
}

fn fix_curly(text: &str) -> String {
    // Add space before { when preceded by identifier, ) or ]
    let mut result = String::with_capacity(text.len() * 2);
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' && i > 0 {
            let prev = bytes[i - 1];
            if (prev.is_ascii_alphanumeric() || prev == b')' || prev == b']') && prev != b'{' && prev != b'(' && prev != b'[' {
                if prev != b' ' && prev != b'\n' {
                    result.push(' ');
                }
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    // Remove double spaces before {
    while result.contains("  {") { result = result.replace("  {", " {"); }
    result
}

fn fix_comma(text: &str) -> String {
    // Remove space before comma, fix double-space after comma
    text.replace(" ,", ",")
        .replace(",  ", ", ")
}

fn fix_comment(text: &str) -> String {
    // Add space after // unless it's /// or //// or empty
    let mut result = String::with_capacity(text.len() * 2);
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 2 < bytes.len() && bytes[i] == b'/' && bytes[i+1] == b'/'
            && bytes[i+2] != b'/' && bytes[i+2] != b' ' && bytes[i+2] != b'\n' {
            result.push_str("// ");
            i += 2;
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

fn fix_blank_rbrace(text: &str) -> String {
    // Remove blank lines before } (squash \n\n} to \n})
    let mut result = text.replace("\n\n}", "\n}");
    // Run again to catch cascading blanks
    while result.contains("\n\n}") {
        result = result.replace("\n\n}", "\n}");
    }
    result
}

fn fix_else(text: &str) -> String {
    text.replace("}\nelse", "} else")
        .replace("}\n    else", "} else")
}

fn fix_lines(text: &str, rules: &std::collections::HashSet<&str>) -> String {
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    let trailing_nl = text.ends_with('\n');

    let mut i = 0;
    while i < lines.len() {
        let cur_empty = lines[i].trim().is_empty();
        let prev_empty = if i > 0 { lines[i - 1].trim().is_empty() } else { false };

        if rules.contains("standard:no-trailing-spaces") {
            lines[i] = lines[i].trim_end().to_string();
        }

        if rules.contains("standard:no-consecutive-blank-lines") {
            if cur_empty && prev_empty {
                lines.remove(i);
                continue;
            }
        }

        i += 1;
    }

    lines.join("\n") + if trailing_nl { "\n" } else { "" }
}
