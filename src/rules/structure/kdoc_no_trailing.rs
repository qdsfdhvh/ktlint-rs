//! standard:no-trailing-spaces-in-kdoc — no trailing spaces in KDoc lines.
use crate::rules::{Rule, Violation};

pub struct KdocNoTrailingSpace;
impl Rule for KdocNoTrailingSpace {
    fn id(&self) -> &'static str {
        "standard:no-trailing-spaces-in-kdoc"
    }
    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        let mut in_kdoc = false;
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("/**") {
                in_kdoc = true;
            }
            if in_kdoc && line.ends_with(' ') {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: line.len(),
                    rule_id: self.id().to_string(),
                    auto_fixable: true,
                    message: "Trailing space in KDoc".into(),
                });
            }
            if t.ends_with("*/") {
                in_kdoc = false;
            }
        }
        v
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        KdocNoTrailingSpace.check(&p.parse(s), s)
    }
    #[test]
    fn ok() {
        assert!(c("/**\n * doc\n */\n").is_empty());
    }
    #[test]
    fn bad() {
        assert!(!c("/**\n * doc \n */\n").is_empty());
    }
}
