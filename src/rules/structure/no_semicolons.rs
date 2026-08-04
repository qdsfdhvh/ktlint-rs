//! `standard:no-semi` parity with ktlint 1.8.

use crate::rules::{Rule, Violation};

pub struct NoSemicolons;

impl Rule for NoSemicolons {
    fn id(&self) -> &'static str {
        "standard:no-semi"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut in_block_comment = false;
        for (line_index, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("/*") {
                in_block_comment = true;
            }
            if in_block_comment {
                if trimmed.ends_with("*/") {
                    in_block_comment = false;
                }
                continue;
            }
            if trimmed.starts_with("//") || trimmed.starts_with("* ") || trimmed == "*" {
                continue;
            }
            // Enum entries (`CLOSE;`, `ApplicationData(0x17);`) require the
            // separator before member declarations — not unnecessary.
            let code = trimmed.trim_end_matches(';');
            let is_enum_entry = !code.is_empty()
                && (code
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
                    || code.ends_with(')'));
            if trimmed.ends_with(';') && trimmed != ";" && !is_enum_entry {
                violations.push(Violation {
                    file: String::new(),
                    line: line_index + 1,
                    col: line.rfind(';').unwrap_or(0) + 1,
                    rule_id: self.id().into(),
                    message: "Unnecessary semicolon".into(),
                    auto_fixable: true,
                });
            }
        }
        violations
    }
}
