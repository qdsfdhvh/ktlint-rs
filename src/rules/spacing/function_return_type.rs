//! standard:function-return-type-spacing — space before `:` in function return type.

use crate::rules::{Rule, Violation};

pub struct FunctionReturnTypeSpacing;

impl Rule for FunctionReturnTypeSpacing {
    fn id(&self) -> &'static str {
        "standard:function-return-type-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl FunctionReturnTypeSpacing {
    fn walk(&self, node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        if node.kind() == "function_declaration" {
            self.check_function(&node, bytes, violations);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, bytes, violations);
            }
        }
    }

    fn check_function(&self, node: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        // Find the `:` between parameter list and return type
        // In function_declaration: `fun name(params)` then optional `:` then return type
        let mut saw_parens = false;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let kind = child.kind();
                if kind == "function_value_parameters" || kind == "(" {
                    saw_parens = true;
                }
                if saw_parens && kind == ":" {
                    let pos = child.start_position();
                    let start_byte = child.start_byte();

                    // In return type context, no space before `:`
                    if start_byte > 0 && bytes[start_byte - 1] == b' ' {
                        violations.push(Violation {
                            file: String::new(),
                            line: pos.row + 1,
                            col: pos.column,
                            rule_id: self.id().to_string(),
                            message: "Unexpected space before \":\" in return type".to_string(),
                            auto_fixable: true,
                        });
                    }

                    // Should have space after
                    let end_byte = child.end_byte();
                    if end_byte < bytes.len() && bytes[end_byte] != b' ' && bytes[end_byte] != b'\n' {
                        violations.push(Violation {
                            file: String::new(),
                            line: pos.row + 1,
                            col: pos.column + 2,
                            rule_id: self.id().to_string(),
                            message: "Missing space after \":\" in return type".to_string(),
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
        FunctionReturnTypeSpacing.check(&tree, source)
    }

    #[test]
    fn valid_return_type() {
        assert!(check("fun foo(): Int = 1\n").is_empty());
    }

    #[test]
    fn space_before_colon_in_return_type() {
        let v = check("fun foo() : Int = 1\n");
        assert!(!v.is_empty());
    }

    #[test]
    fn no_return_type_is_ok() {
        assert!(check("fun foo() { }\n").is_empty());
    }
}
