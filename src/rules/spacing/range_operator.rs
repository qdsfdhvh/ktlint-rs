//! standard:spacing-around-range-operator — spaces around .. operator.
use crate::rules::{Rule, Violation};

pub struct RangeOperatorSpacing;

impl Rule for RangeOperatorSpacing {
    fn id(&self) -> &'static str {
        "standard:spacing-around-range-operator"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == ".." {
                let offset = node.start_byte();
                let pos = node.start_position();
                if offset > 0 && bytes[offset - 1] == b' ' {
                    violations.push(Violation {
                        file: String::new(),
                        line: pos.row + 1,
                        col: pos.column + 1,
                        rule_id: self.id().into(),
                        message: "Unexpected space before \"..\"".into(),
                        auto_fixable: true,
                    });
                }
                if node.end_byte() < bytes.len() && bytes[node.end_byte()] == b' ' {
                    violations.push(Violation {
                        file: String::new(),
                        line: pos.row + 1,
                        col: pos.column + 3,
                        rule_id: self.id().into(),
                        message: "Unexpected space after \"..\"".into(),
                        auto_fixable: true,
                    });
                }
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
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        let t = p.parse(s);
        RangeOperatorSpacing.check(&t, s)
    }
    #[test]
    fn range_ok() {
        assert!(check("for (i in 1..10)\n").is_empty());
    }
    #[test]
    fn space_before_range() {
        let v = check("for (i in 1 ..10)\n");
        assert!(!v.is_empty());
        assert!(v.iter().any(|x| x.message.contains("before")));
    }
}
