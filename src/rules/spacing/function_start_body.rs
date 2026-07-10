//! standard:function-start-of-body-spacing — newline before `{` in function body.

use crate::rules::{Rule, Violation};

pub struct FunctionStartOfBodySpacing;

impl Rule for FunctionStartOfBodySpacing {
    fn id(&self) -> &'static str {
        "standard:function-start-of-body-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl FunctionStartOfBodySpacing {
    fn walk(&self, node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        if node.kind() == "function_body" {
            self.check_body(&node, bytes, violations);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, bytes, violations);
            }
        }
    }

    fn check_body(&self, node: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        // function_body: `{ stmts }` or `= expr`
        // Find the opening `{`
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "{" {
                    let start_byte = child.start_byte();
                    if start_byte > 0 && bytes[start_byte - 1] != b'\n' && bytes[start_byte - 1] != b' ' {
                        let pos = child.start_position();
                        // This is a compact function body like `fun foo() { }` — that's OK
                        // Only flag if the previous token is not space or newline
                        violations.push(Violation {
                            file: String::new(),
                            line: pos.row + 1,
                            col: pos.column + 1,
                            rule_id: self.id().to_string(),
                            message: "Line break expected before opening brace".to_string(),
                            auto_fixable: true,
                        });
                    }
                    break;
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
        FunctionStartOfBodySpacing.check(&tree, source)
    }

    #[test]
    fn newline_before_brace() {
        assert!(check("fun foo() {\n}\n").is_empty());
    }

    #[test]
    fn expression_body_no_brace() {
        // Expression body: `fun foo() = 1` — no violations for missing brace
        let v = check("fun foo() = 1\n");
        assert!(v.is_empty());
    }
}
