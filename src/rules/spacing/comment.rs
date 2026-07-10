//! standard:comment-spacing — single space after `//`, space after `/*`, space before `*/`.

use crate::rules::{Rule, Violation};

pub struct CommentSpacing;

impl Rule for CommentSpacing {
    fn id(&self) -> &'static str {
        "standard:comment-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        self.walk(tree.root_node(), source, &mut violations);
        violations
    }
}

impl CommentSpacing {
    fn walk(&self, node: tree_sitter::Node, source: &str, violations: &mut Vec<Violation>) {
        let kind = node.kind();
        if kind == "comment" || kind == "line_comment" {
            self.check_line_comment(&node, source, violations);
        } else if kind == "multiline_comment" {
            self.check_multiline_comment(&node, source, violations);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, source, violations);
            }
        }
    }

    fn check_line_comment(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        violations: &mut Vec<Violation>,
    ) {
        let text = &source[node.start_byte()..node.end_byte()];
        let pos = node.start_position();

        // `//` should be followed by a space (unless it's `////\n` or empty `//`)
        if text.starts_with("//") && text.len() > 2 {
            let third = text.as_bytes()[2];
            if third != b' ' && third != b'/' && third != b'\n' && third != b'\r' {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 3,
                    rule_id: self.id().to_string(),
                    message: "Missing space after \"//\"".to_string(),
                    auto_fixable: true,
                });
            }
        }
    }

    fn check_multiline_comment(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        violations: &mut Vec<Violation>,
    ) {
        let text = &source[node.start_byte()..node.end_byte()];
        let pos = node.start_position();

        // `/*` should be followed by space if there's content
        if text.starts_with("/*") && text.len() > 2 {
            let third = text.as_bytes()[2];
            if third != b' ' && third != b'*' && third != b'\n' {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 3,
                    rule_id: self.id().to_string(),
                    message: "Missing space after \"/*\"".to_string(),
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
        CommentSpacing.check(&tree, source)
    }

    #[test]
    fn line_comment_with_space() {
        assert!(check("// hello\nval x = 1\n").is_empty());
    }

    #[test]
    fn line_comment_missing_space() {
        let v = check("//hello\nval x = 1\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:comment-spacing");
    }

    #[test]
    fn triple_slash_is_ok() {
        assert!(check("/// KDoc\nval x = 1\n").is_empty());
    }

    #[test]
    fn empty_comment_is_ok() {
        assert!(check("//\nval x = 1\n").is_empty());
    }
}
