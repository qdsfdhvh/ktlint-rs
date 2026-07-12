//! standard:no-semicolons — unnecessary semicolons
use crate::rules::{Rule, Violation};
pub struct NoSemicolons;
impl Rule for NoSemicolons {
    fn id(&self) -> &'static str {
        "standard:no-semicolons"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        s.lines()
            .enumerate()
            .filter(|(_, l)| l.trim().ends_with(';') && !l.trim().starts_with("//"))
            .map(|(i, _)| Violation {
                file: String::new(),
                line: i + 1,
                col: 1,
                rule_id: self.id().into(),
                message: "Unnecessary semicolon".into(),
                auto_fixable: true,
            })
            .collect()
    }
}
