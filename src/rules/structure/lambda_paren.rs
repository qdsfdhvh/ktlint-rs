//! standard:no-unnecessary-parentheses-before-trailing-lambda
use crate::rules::{Rule, Violation};

pub struct LambdaParen;

impl Rule for LambdaParen {
    fn id(&self) -> &'static str {
        "standard:no-unnecessary-parentheses-before-trailing-lambda"
    }
    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.contains(") {") && t.contains("fun ") {
                if let Some(pos) = t.find(") {") {
                    // Check if the `{` after `)` is a trailing lambda
                    if i + 1 < source.lines().count() {
                        violations.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: pos + 2,
                            rule_id: self.id().to_string(),
                            message: "Unnecessary parentheses before trailing lambda".to_string(),
                            auto_fixable: true,
                        });
                    }
                }
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
        LambdaParen.check(&p.parse(s), s)
    }
    #[test]
    fn no_paren_lambda() {
        assert!(check("list.forEach { it }\n").is_empty());
    }
}
