//! standard:op-spacing — ensures single spaces around operators.
//!
//! Checks: `=`, `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`
//! Each operator should have exactly one space before and after.

use crate::rules::{Rule, Violation};

pub struct OperatorSpacing;

// Known operator node kinds in tree-sitter-kotlin-sg
const OPERATORS: &[&str] = &[
    "=", "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=", "&&", "||",
];

impl Rule for OperatorSpacing {
    fn id(&self) -> &'static str {
        "standard:op-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk_and_check(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl OperatorSpacing {
    fn walk_and_check(
        &self,
        node: tree_sitter::Node,
        bytes: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        let kind = node.kind();
        if OPERATORS.contains(&kind) {
            // = in parameter default values: `val x: Int = 5` — check context
            // = in property declarations: check context
            // All other operators: simple spacing check
            self.check_operator(&node, bytes, violations);
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk_and_check(child, bytes, violations);
            }
        }
    }

    fn check_operator(
        &self,
        node: &tree_sitter::Node,
        bytes: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        let pos = node.start_position();
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();

        // Check space before
        if start_byte > 0 && bytes[start_byte - 1] != b' ' {
            violations.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: self.id().to_string(),
                message: format!("Missing space before \"{}\"", node.kind()),
                auto_fixable: true,
            });
        }

        // Check space after
        if end_byte < bytes.len() && bytes[end_byte] != b' ' && bytes[end_byte] != b'\n' {
            violations.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: self.id().to_string(),
                message: format!("Missing space after \"{}\"", node.kind()),
                auto_fixable: true,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        OperatorSpacing.check(&tree, source)
    }

    #[test]
    fn valid_operator_spacing() {
        let source = "val x = 1 + 2\n";
        assert!(check(source).is_empty());
    }

    #[test]
    fn missing_space_around_op() {
        let v = check("val x= 1 + 2\n");
        assert!(!v.is_empty());
    }
}
