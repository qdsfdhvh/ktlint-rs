//! standard:parameter-list-spacing — no extra spaces in parameter lists.

use crate::rules::{Rule, Violation};

pub struct ParameterListSpacing;

impl Rule for ParameterListSpacing {
    fn id(&self) -> &'static str {
        "standard:parameter-list-spacing"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // Check for parameters inside function/constructor declarations
            if trimmed.contains('(') && trimmed.contains(')') {
                // Detect patterns like `fun foo( x: Int, y: String)` or `fun foo(x : Int)`
                // These are already covered by paren-spacing and colon-spacing,
                // but this rule catches the specific case of double spaces in param lists
                if let Some(paren_start) = trimmed.find('(') {
                    if let Some(paren_end) = trimmed.rfind(')') {
                        let params = &trimmed[paren_start + 1..paren_end];
                        // Check for multiple consecutive spaces
                        if params.contains("  ") {
                            violations.push(Violation {
                                file: String::new(),
                                line: i + 1,
                                col: paren_start + 2,
                                rule_id: self.id().to_string(),
                                message: "Extra spaces in parameter list".to_string(),
                                auto_fixable: true,
                            });
                        }
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

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        ParameterListSpacing.check(&tree, source)
    }

    #[test]
    fn normal_params_ok() {
        assert!(check("fun foo(a: Int, b: String)\n").is_empty());
    }

    #[test]
    fn double_space_in_params() {
        let v = check("fun foo( a: Int,  b: String)\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:parameter-list-spacing");
    }

    #[test]
    fn empty_params_ok() {
        assert!(check("fun foo()\n").is_empty());
    }
}
