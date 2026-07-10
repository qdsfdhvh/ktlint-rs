//! standard:block-comment-initial-star-blank-line — star alignment in block comments.
use crate::rules::{Rule, Violation};

pub struct BlockCommentStar;

impl Rule for BlockCommentStar {
    fn id(&self) -> &'static str { "standard:block-comment-initial-star-blank-line" }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        let mut in_block = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("/*") && !trimmed.ends_with("*/") {
                in_block = true;
            }
            if in_block && trimmed.starts_with('*') && !trimmed.starts_with("*/") && !trimmed.starts_with("/**") {
                if !trimmed.starts_with("* ") && trimmed.len() > 1 {
                    violations.push(Violation {
                        file: String::new(), line: i + 1, col: 1,
                        rule_id: self.id().to_string(),
                        message: "Block comment star should be followed by space".to_string(),
                        auto_fixable: true,
                    });
                }
            }
            if trimmed.ends_with("*/") { in_block = false; }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*; use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> { let mut p=KotlinParser::new(); BlockCommentStar.check(&p.parse(s), s) }
    #[test] fn ok() { assert!(check("/*\n * hello\n */\n").is_empty()); }
    #[test] fn no_space() { let v=check("/*\n *hello\n */\n"); assert!(!v.is_empty()); }
}
