//! standard:no-multi-spaces — flag multiple consecutive spaces outside strings.
use crate::rules::{Rule, Violation};

pub struct NoMultiSpaces;

impl Rule for NoMultiSpaces {
    fn id(&self) -> &'static str { "standard:no-multi-spaces" }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("  ") && !trimmed.starts_with("//") && !trimmed.starts_with('*') {
                // Don't flag leading indentation (handled by indent rule)
                let stripped = line.trim_start();
                if stripped.contains("  ") {
                    violations.push(Violation {
                        file: String::new(), line: i + 1, col: 1,
                        rule_id: self.id().to_string(),
                        message: "Multiple consecutive spaces".to_string(),
                        auto_fixable: true,
                    });
                }
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*; use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> { let mut p=KotlinParser::new(); NoMultiSpaces.check(&p.parse(s), s) }
    #[test] fn single_space() { assert!(check("val x = 1\n").is_empty()); }
    #[test] fn multi_space() { let v=check("val x  = 1\n"); assert!(!v.is_empty()); }
    #[test] fn indentation_ok() { assert!(check("    val x = 1\n").is_empty()); }
}
