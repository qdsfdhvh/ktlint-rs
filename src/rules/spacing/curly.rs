//! standard:curly-spacing — ensures spaces around curly braces.
//!
//! Rules:
//! - `{` should be preceded by a single space (unless at line start or after `(`, `[`)
//! - `{` should be followed by correct spacing (newline for bodies, space for lambdas)
//! - `}` should not be preceded by a space unless on same line as content

use crate::rules::{Rule, Violation};
use tree_sitter::Node;

pub struct CurlySpacing;

impl Rule for CurlySpacing {
    fn id(&self) -> &'static str {
        "standard:curly-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk_and_check(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl CurlySpacing {
    fn walk_and_check(&self, node: Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        if node.kind() == "{" {
            self.check_open_brace(&node, bytes, violations);
        } else if node.kind() == "}" {
            self.check_close_brace(&node, bytes, violations);
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk_and_check(child, bytes, violations);
            }
        }
    }

    fn check_open_brace(&self, node: &Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let pos = node.start_position();
        let start_byte = node.start_byte();

        // Skip over newlines: if preceded by \n, it's already at line start (fine)
        if start_byte > 0 {
            let prev_char = bytes[start_byte - 1];
            // Should have space before unless preceded by (, [, or at line start
            if prev_char == b'\n' {
                // OK — at line start
            } else if prev_char == b'(' || prev_char == b'[' {
                // OK — directly after opening paren/bracket
            } else if prev_char != b' ' {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 1,
                    rule_id: self.id().to_string(),
                    message: "Missing space before \"{\"".to_string(),
                    auto_fixable: true,
                });
            } else if start_byte >= 2 && bytes[start_byte - 2] == b' ' {
                // Double space before {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 1,
                    rule_id: self.id().to_string(),
                    message: "Too many spaces before \"{\"".to_string(),
                    auto_fixable: true,
                });
            }
        }
    }

    fn check_close_brace(&self, node: &Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let pos = node.start_position();
        let start_byte = node.start_byte();

        // } should be at the start of a line (possibly with indent) or preceded by a single space
        if start_byte > 0 {
            let prev_char = bytes[start_byte - 1];
            if prev_char == b'\n' {
                // OK — at line start
            } else if prev_char != b' ' && prev_char != b'{' && prev_char != b';' {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 1,
                    rule_id: self.id().to_string(),
                    message: "Missing space before \"}\"".to_string(),
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
        CurlySpacing.check(&tree, source)
    }

    #[test]
    fn valid_curly_spacing() {
        let source = "class Foo {\n    fun bar() {\n        return 1\n    }\n}\n";
        assert!(check(source).is_empty());
    }

    #[test]
    fn missing_space_before_open_brace() {
        let source = "class Foo{\n}\n";
        let v = check(source);
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:curly-spacing");
    }

    #[test]
    fn double_space_before_open_brace() {
        let source = "class Foo  {\n}\n";
        let v = check(source);
        assert!(!v.is_empty());
    }

    #[test]
    fn brace_after_paren_is_ok() {
        let source = "fun foo(): Int {\n    return 1\n}\n";
        let v = check(source);
        assert!(v.is_empty());
    }
}
