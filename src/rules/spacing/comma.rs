//! standard:comma-spacing — aligned with JVM ktlint SpacingAroundCommaRule.
//! No space before comma. Single space after comma (skip if next is ), ], >)

use crate::rules::{Rule, Violation};

pub struct CommaSpacing;
impl Rule for CommaSpacing {
    fn id(&self) -> &'static str {
        "standard:comma-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        Self::walk(tree.root_node(), bytes, &mut violations);
        violations
    }
}
impl CommaSpacing {
    fn walk(node: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
        if node.kind() == "," {
            Self::check(&node, bytes, v);
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                Self::walk(c, bytes, v);
            }
        }
    }

    fn check(node: &tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
        let pos = node.start_position();
        let s = node.start_byte();
        let e = node.end_byte();

        // 1. No space before comma (unless after comment on same line)
        if s > 0 && bytes[s - 1] == b' ' {
            // Check if this is a comma after a comment (on new line) — skip
            if s < 2 || bytes[s - 2] != b'\n' {
                v.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column,
                    rule_id: "standard:comma-spacing".into(),
                    auto_fixable: true,
                    message: "Unexpected spacing before \",\"".into(),
                });
            }
        }

        // 2. Space after comma (skip if next is ), ], > or newline)
        if e < bytes.len() {
            let next = bytes[e];
            if next == b')' || next == b']' || next == b'>' || next == b'\n' || next == b'\r' {
                return;
            }
            if next != b' ' {
                v.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 2,
                    rule_id: "standard:comma-spacing".into(),
                    auto_fixable: true,
                    message: "Missing spacing after \",\"".into(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        CommaSpacing.check(&p.parse(s), s)
    }
    #[test]
    fn ok() {
        assert!(c("fun f(a: Int, b: String)\n").is_empty());
    }
    #[test]
    fn space_before() {
        let v = c("fun f(a , b)\n");
        assert!(v.iter().any(|x| x.message.contains("before")));
    }
    #[test]
    fn no_space_after() {
        let v = c("fun f(a,b)\n");
        assert!(v.iter().any(|x| x.message.contains("after")));
    }
    #[test]
    fn trailing_comma() {
        assert!(c("fun f(a: Int,)\n").is_empty());
    }
    #[test]
    fn after_paren_skip() {
        assert!(c("mapOf(1,)\n").is_empty());
    }
}
