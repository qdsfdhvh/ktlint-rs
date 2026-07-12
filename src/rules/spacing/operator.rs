//! standard:op-spacing — single spaces around operators. Skips comments/generics.
use crate::rules::{Rule, Violation};

pub struct OperatorSpacing;
const OPERATORS: &[&str] = &[
    "=", "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=", "&&", "||",
];

impl Rule for OperatorSpacing {
    fn id(&self) -> &'static str {
        "standard:op-spacing"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        Self::walk(tree.root_node(), bytes, &mut v);
        v
    }
}
impl OperatorSpacing {
    fn walk(node: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
        let kind = node.kind();
        if OPERATORS.contains(&kind) {
            if !Self::in_comment(&node)
                && !((kind == "<" || kind == ">") && Self::is_generic(&node))
            {
                Self::check_op(&node, bytes, v);
            }
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                Self::walk(c, bytes, v);
            }
        }
    }
    fn in_comment(node: &tree_sitter::Node) -> bool {
        let mut cur = Some(*node);
        while let Some(n) = cur {
            if matches!(n.kind(), "comment" | "multiline_comment" | "line_comment") {
                return true;
            }
            cur = n.parent();
        }
        false
    }
    fn is_generic(node: &tree_sitter::Node) -> bool {
        node.parent().map_or(false, |p| {
            matches!(
                p.kind(),
                "type_arguments" | "type_parameters" | "type_projection"
            )
        })
    }
    fn check_op(node: &tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
        let pos = node.start_position();
        let s = node.start_byte();
        let e = node.end_byte();
        if s > 0 && bytes[s - 1] != b' ' && bytes[s - 1] != b'(' && bytes[s - 1] != b'\n' {
            v.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: "standard:op-spacing".into(),
                auto_fixable: true,
                message: format!("Missing space before \"{}\"", node.kind()),
            });
        }
        if e < bytes.len() && bytes[e] != b' ' && bytes[e] != b')' && bytes[e] != b'\n' {
            v.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: "standard:op-spacing".into(),
                auto_fixable: true,
                message: format!("Missing space after \"{}\"", node.kind()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        OperatorSpacing.check(&p.parse(s), s)
    }
    #[test]
    fn ok() {
        assert!(!c("val x=1+2\n").is_empty());
    }
    #[test]
    fn generic_skip() {
        let v = c("val x:List<String>=listOf()\n");
        assert_eq!(
            v.iter()
                .filter(|x| x.rule_id == "standard:op-spacing" && x.message.contains("<"))
                .count(),
            0
        );
    }
}
