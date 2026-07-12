//! standard:no-semicolons — unnecessary semicolons
use crate::rules::{Rule, Violation};
pub struct NoSemicolons;
impl Rule for NoSemicolons {
    fn id(&self) -> &'static str {
        "standard:no-semicolons"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut in_block_comment = false;

        for (i, line) in s.lines().enumerate() {
            let trimmed = line.trim();

            // Track block comment state
            if trimmed.starts_with("/*") {
                in_block_comment = true;
            }
            if in_block_comment {
                if trimmed.ends_with("*/") {
                    in_block_comment = false;
                }
                continue;
            }

            // Skip line comments and block comment continuation lines (common * prefix)
            if trimmed.starts_with("//") || trimmed.starts_with("* ") || trimmed == "*" {
                continue;
            }

            if trimmed.ends_with(';') {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Unnecessary semicolon".into(),
                    auto_fixable: true,
                });
            }
        }

        violations
}
}
