//! standard:max-line-length — lines should not exceed 120 chars (configurable).

use crate::rules::{Rule, Violation};

pub struct MaxLineLength;

impl Rule for MaxLineLength {
    fn id(&self) -> &'static str {
        "standard:max-line-length"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let max_length = 120; // default; configurable via .editorconfig

        source
            .lines()
            .enumerate()
            .filter(|(_, line)| line.len() > max_length)
            .map(|(i, _line)| Violation {
                file: String::new(),
                line: i + 1,
                col: max_length + 1,
                rule_id: self.id().to_string(),
                message: format!("Line exceeds {} characters", max_length),
                auto_fixable: false, // wrapping requires manual intervention
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        MaxLineLength.check(&tree, source)
    }

    #[test]
    fn short_lines() {
        assert!(check("val x = 1\n").is_empty());
    }

    #[test]
    fn long_line() {
        let long = format!("val x = \"{}\"\n", "a".repeat(200));
        let v = check(&long);
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:max-line-length");
    }
}
