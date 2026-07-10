//! standard:paren-spacing — no space after `(`, no space before `)`.

use crate::rules::{Rule, Violation};

pub struct ParenSpacing;

impl Rule for ParenSpacing {
    fn id(&self) -> &'static str {
        "standard:paren-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl ParenSpacing {
    fn walk(&self, node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let kind = node.kind();
        if kind == "(" {
            self.check_open_paren(&node, bytes, violations);
        } else if kind == ")" {
            self.check_close_paren(&node, bytes, violations);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, bytes, violations);
            }
        }
    }

    fn check_open_paren(
        &self,
        node: &tree_sitter::Node,
        bytes: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        let end_byte = node.end_byte();
        let pos = node.start_position();

        if end_byte < bytes.len() && bytes[end_byte] == b' ' {
            violations.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 2,
                rule_id: self.id().to_string(),
                message: "Unexpected space after \"(\"".to_string(),
                auto_fixable: true,
            });
        }
    }

    fn check_close_paren(
        &self,
        node: &tree_sitter::Node,
        bytes: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        let start_byte = node.start_byte();
        let pos = node.start_position();

        if start_byte > 1 && bytes[start_byte - 1] == b' ' && bytes[start_byte - 2] != b' ' && bytes[start_byte - 2] != b'\n' {
            // Don't flag if the space is part of an aligned parameter list
            // (this is a simplification — real ktlint checks for alignment)
            violations.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column,
                rule_id: self.id().to_string(),
                message: "Unexpected space before \")\"".to_string(),
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
        ParenSpacing.check(&tree, source)
    }

    #[test]
    fn valid_paren_spacing() {
        assert!(check("fun foo(a: Int)\n").is_empty());
    }

    #[test]
    fn space_after_open_paren() {
        let v = check("fun foo( a: Int)\n");
        assert!(!v.is_empty());
        assert!(v.iter().any(|x| x.message.contains("after")));
    }

    #[test]
    fn space_before_close_paren() {
        let v = check("fun foo(a: Int )\n");
        assert!(!v.is_empty());
        assert!(v.iter().any(|x| x.message.contains("before")));
    }

    #[test]
    fn empty_parens_ok() {
        assert!(check("fun foo()\n").is_empty());
    }
}
