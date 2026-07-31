//! `standard:range-spacing` parity with ktlint 1.8.

use crate::rules::{Rule, Violation};

pub struct RangeOperatorSpacing;

impl Rule for RangeOperatorSpacing {
    fn id(&self) -> &'static str {
        "standard:range-spacing"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == ".." {
                let offset = node.start_byte();
                let pos = node.start_position();
                let before = offset > 0 && bytes[offset - 1].is_ascii_whitespace();
                let after =
                    node.end_byte() < bytes.len() && bytes[node.end_byte()].is_ascii_whitespace();
                let (column, message) = match (before, after) {
                    (true, true) => (pos.column + 1, "Unexpected spacing around \"..\""),
                    (true, false) => (pos.column, "Unexpected spacing before \"..\""),
                    (false, true) => (pos.column + 3, "Unexpected spacing after \"..\""),
                    (false, false) => {
                        for index in (0..node.child_count()).rev() {
                            if let Some(child) = node.child(index) {
                                stack.push(child);
                            }
                        }
                        continue;
                    }
                };
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: column,
                    rule_id: self.id().into(),
                    message: message.into(),
                    auto_fixable: true,
                });
            }
            for index in (0..node.child_count()).rev() {
                if let Some(child) = node.child(index) {
                    stack.push(child);
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
        RangeOperatorSpacing.check(&parser.parse(source), source)
    }

    #[test]
    fn accepts_compact_range() {
        assert!(check("val range = 1..10\n").is_empty());
    }

    #[test]
    fn groups_spacing_around_range() {
        let violations = check("val range = 1 .. 10\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].message, "Unexpected spacing around \"..\"");
    }
}
