//! standard:parameter-list-spacing — no extra spaces in parameter lists.
use crate::rules::{Rule, Violation};

pub struct ParameterListSpacing;
impl Rule for ParameterListSpacing {
    fn id(&self) -> &'static str { "standard:parameter-list-spacing" }
    fn auto_fixable(&self) -> bool { true }
    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains('(') && trimmed.contains(')') {
                if let Some(paren_start) = trimmed.find('(') {
                    if let Some(paren_end) = trimmed.rfind(')') {
                        if paren_end > paren_start + 1 {
                            let params = &trimmed[paren_start + 1..paren_end];
                            if params.contains("  ") {
                                violations.push(Violation { file: String::new(), line: i + 1, col: paren_start + 2,
                                    rule_id: self.id().to_string(), auto_fixable: true,
                                    message: "Extra spaces in parameter list".to_string() });
                            }
                        }
                    }
                }
            }
        }
        violations
    }
}

#[cfg(test)] mod tests { use super::*; use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> { let mut p=KotlinParser::new(); ParameterListSpacing.check(&p.parse(s), s) }
    #[test] fn ok() { assert!(c("fun foo(a: Int, b: String)\n").is_empty()); }
    #[test] fn double() { assert!(!c("fun foo( a: Int,  b: String)\n").is_empty()); }
    #[test] fn empty() { assert!(c("fun foo()\n").is_empty()); }
}
