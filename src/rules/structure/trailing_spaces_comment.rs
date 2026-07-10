//! standard:no-trailing-spaces-in-block-comment — trailing spaces inside block comments.
use crate::rules::{Rule, Violation};

pub struct TrailingSpacesInComment;

impl Rule for TrailingSpacesInComment {
    fn id(&self) -> &'static str {
        "standard:no-trailing-spaces-in-block-comment"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        let mut in_block = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("/*") {
                in_block = true;
            }

            if in_block && line.ends_with(' ') {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: line.len(),
                    rule_id: self.id().to_string(),
                    message: "Trailing space in block comment".to_string(),
                    auto_fixable: true,
                });
            }

            if trimmed.ends_with("*/") {
                in_block = false;
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        TrailingSpacesInComment.check(&p.parse(s), s)
    }
    #[test]
    fn ok() {
        assert!(check("/*\n * hello\n */\n").is_empty());
    }
    #[test]
    fn trailing() {
        let v = check("/*\n * hello \n */\n");
        assert!(!v.is_empty());
    }
}
