//! standard:trailing-comma — enforce trailing comma on call site parameter lists (configurable).

use crate::rules::{Rule, Violation};

pub struct TrailingComma;

impl Rule for TrailingComma {
    fn id(&self) -> &'static str {
        "standard:trailing-comma"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl TrailingComma {
    fn walk(&self, node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        if node.kind() == "value_arguments" || node.kind() == "class_parameters" {
            self.check_trailing_comma(&node, bytes, violations);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, bytes, violations);
            }
        }
    }

    fn check_trailing_comma(
        &self,
        node: &tree_sitter::Node,
        _bytes: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        // Get the last non-`)` child. If it has content (not just comments),
        // there should be a trailing comma before the closing `)`.
        // This is a simplified check — full implementation would consider line breaks.

        let child_count = node.child_count();
        if child_count < 3 {
            // Just `()` — no parameters
            return;
        }

        // Find the `)` position
        let mut close_paren_pos = None;
        for i in (0..child_count).rev() {
            if let Some(child) = node.child(i) {
                if child.kind() == ")" {
                    close_paren_pos = Some((i, child.start_byte()));
                    break;
                }
            }
        }

        if let Some((ci, cp_byte)) = close_paren_pos {
            if ci > 0 {
                // Check if there's content on a different line before `)`
                let mut prev_line = None;
                for i in (0..ci).rev() {
                    if let Some(child) = node.child(i) {
                        let child_line = child.start_position().row;
                        let cp_line = node
                            .child(ci)
                            .map(|c| c.start_position().row)
                            .unwrap_or(child_line);

                        if child_line != cp_line {
                            // Parameter is on a different line — check for trailing comma
                            let child_end = child.end_byte();
                            if child_end < cp_byte
                                && child.kind() != ","
                                && child.kind() != "comment"
                                && child.kind() != "multiline_comment"
                            {
                                violations.push(Violation {
                                    file: String::new(),
                                    line: cp_line + 1,
                                    col: 1,
                                    rule_id: self.id().to_string(),
                                    message: "Missing trailing comma before \")\"".to_string(),
                                    auto_fixable: true,
                                });
                            }
                            break;
                        }
                        prev_line = Some(child_line);
                    }
                }
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
        TrailingComma.check(&tree, source)
    }

    #[test]
    fn single_line_no_trailing_comma_needed() {
        assert!(check("fun foo(a: Int, b: String)\n").is_empty());
    }
}
