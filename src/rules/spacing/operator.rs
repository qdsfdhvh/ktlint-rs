//! standard:op-spacing — ensures single spaces around operators.
//! Skips < > when used as generic type brackets.

use crate::rules::{Rule, Violation};

pub struct OperatorSpacing;

const OPERATORS: &[&str] = &["=", "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=", "&&", "||"];

impl Rule for OperatorSpacing {
    fn id(&self) -> &'static str { "standard:op-spacing" }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk_and_check(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl OperatorSpacing {
    fn walk_and_check(&self, node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let kind = node.kind();
        if OPERATORS.contains(&kind) {
            // Skip < > when used as generic type brackets
            if (kind == "<" || kind == ">") && self.is_generic_bracket(&node) {
                // skip
            } else {
                self.check_operator(&node, bytes, violations);
            }
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) { self.walk_and_check(child, bytes, violations); }
        }
    }

    fn is_generic_bracket(&self, node: &tree_sitter::Node) -> bool {
        if let Some(parent) = node.parent() {
            matches!(parent.kind(), "type_arguments" | "type_parameters" | "type_projection")
        } else { false }
    }

    fn check_operator(&self, node: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let pos = node.start_position();
        let s = node.start_byte();
        let e = node.end_byte();

        // Space before: skip if preceded by ( (function call) or newline
        if s > 0 && bytes[s - 1] != b' ' && bytes[s - 1] != b'(' && bytes[s - 1] != b'\n' {
            violations.push(Violation {
                file: String::new(), line: pos.row + 1, col: pos.column + 1,
                rule_id: self.id().to_string(), auto_fixable: true,
                message: format!("Missing space before \"{}\"", node.kind()),
            });
        }
        // Space after: skip if followed by ) or newline
        if e < bytes.len() && bytes[e] != b' ' && bytes[e] != b')' && bytes[e] != b'\n' {
            violations.push(Violation {
                file: String::new(), line: pos.row + 1, col: pos.column + 1,
                rule_id: self.id().to_string(), auto_fixable: true,
                message: format!("Missing space after \"{}\"", node.kind()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*; use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> { let mut p=KotlinParser::new(); OperatorSpacing.check(&p.parse(s), s) }

    #[test] fn valid() { assert!(check("val x = 1 + 2\n").is_empty()); }
    #[test] fn missing() { assert!(!check("val x= 1 + 2\n").is_empty()); }
    #[test] fn generic_not_flagged() {
        assert!(check("val x: List<String> = listOf()\n").is_empty());
        assert!(check("fun <T> foo(): T\n").is_empty());
    }
}
