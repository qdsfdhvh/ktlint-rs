//! standard:comma-spacing — no space before comma, exactly one space after.

use crate::rules::{Rule, Violation};

pub struct CommaSpacing;

impl Rule for CommaSpacing {
    fn id(&self) -> &'static str {
        "standard:comma-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl CommaSpacing {
    fn walk(&self, node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        if node.kind() == "," {
            self.check_comma(&node, bytes, violations);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, bytes, violations);
            }
        }
    }

    fn check_comma(&self, node: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let pos = node.start_position();
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();

        // No space before comma
        if start_byte > 0 && bytes[start_byte - 1] == b' ' {
            violations.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column,
                rule_id: self.id().to_string(),
                message: "Unexpected space before \",\"".to_string(),
                auto_fixable: true,
            });
        }

        // Exactly one space after comma (unless followed by newline)
        if end_byte < bytes.len() {
            let after = bytes[end_byte];
            if after == b' ' {
                // Check for double space
                if end_byte + 1 < bytes.len() && bytes[end_byte + 1] == b' ' {
                    violations.push(Violation {
                        file: String::new(),
                        line: pos.row + 1,
                        col: pos.column + 2,
                        rule_id: self.id().to_string(),
                        message: "Too many spaces after \",\"".to_string(),
                        auto_fixable: true,
                    });
                }
                // Single space after comma is correct
            } else if after != b'\n' && after != b'\r' {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 2,
                    rule_id: self.id().to_string(),
                    message: "Missing space after \",\"".to_string(),
                    auto_fixable: true,
                });
            }
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
        CommaSpacing.check(&tree, source)
    }

    #[test]
    fn valid_comma_spacing() {
        assert!(check("fun foo(a: Int, b: String)\n").is_empty());
    }

    #[test]
    fn space_before_comma() {
        let v = check("fun foo(a: Int , b: String)\n");
        assert!(!v.is_empty());
    }

    #[test]
    fn missing_space_after_comma() {
        let v = check("fun foo(a: Int,b: String)\n");
        assert!(!v.is_empty());
    }
}
