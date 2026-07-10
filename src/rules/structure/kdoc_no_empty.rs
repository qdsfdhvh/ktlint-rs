//! standard:kdoc-no-empty-first-line — no blank first line in KDoc.
use crate::rules::{Rule, Violation};

pub struct KdocNoEmptyFirstLine;
impl Rule for KdocNoEmptyFirstLine {
    fn id(&self) -> &'static str { "standard:kdoc-no-empty-first-line" }
    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim() == "/**" && i+1 < lines.len() && lines[i+1].trim().is_empty() {
                v.push(Violation { file: String::new(), line: i+2, col: 1,
                    rule_id: self.id().to_string(), auto_fixable: true,
                    message: "Unexpected blank line after KDoc opening".into(),
                });
            }
        }
        v
    }
}
#[cfg(test)] mod tests { use super::*; use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> { let mut p=KotlinParser::new(); KdocNoEmptyFirstLine.check(&p.parse(s), s) }
    #[test] fn ok() { assert!(c("/**\n * doc\n */\nclass A\n").is_empty()); }
    #[test] fn bad() { assert!(!c("/**\n\n * doc\n */\nclass A\n").is_empty()); }
}
