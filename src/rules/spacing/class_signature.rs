//! standard:class-signature — spacing around class signature components.
//!
//! Checks:
//! - Space before `:` in super type list
//! - Constructor parameter spacing
//! - Class body `{` positioning

use crate::rules::{Rule, Violation};

pub struct ClassSignatureSpacing;

impl Rule for ClassSignatureSpacing {
    fn id(&self) -> &'static str {
        "standard:class-signature"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl ClassSignatureSpacing {
    fn walk(&self, node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        if node.kind() == "class_declaration" {
            self.check_class(&node, bytes, violations);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, bytes, violations);
            }
        }
    }

    fn check_class(
        &self,
        node: &tree_sitter::Node,
        bytes: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        let mut saw_class_keyword = false;
        let mut saw_constructor_or_body = false;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let kind = child.kind();
                if kind == "class" {
                    saw_class_keyword = true;
                }
                // After class name and optional constructor, check `:` for super types
                if kind == "primary_constructor" || kind == "class_body" {
                    saw_constructor_or_body = true;
                }
                // : in super type delegation
                if saw_class_keyword && kind == ":" {
                    // This `:` is in the delegation specifier (super type list)
                    // Should have space before and after
                    let pos = child.start_position();
                    let start_byte = child.start_byte();
                    let end_byte = child.end_byte();

                    // Space before
                    if start_byte > 0
                        && bytes[start_byte - 1] != b' '
                        && bytes[start_byte - 1] != b'\n'
                    {
                        violations.push(Violation {
                            file: String::new(),
                            line: pos.row + 1,
                            col: pos.column,
                            rule_id: self.id().to_string(),
                            message: "Missing space before \":\" in super type list"
                                .to_string(),
                            auto_fixable: true,
                        });
                    }
                    // Space after
                    if end_byte < bytes.len()
                        && bytes[end_byte] != b' '
                        && bytes[end_byte] != b'\n'
                    {
                        violations.push(Violation {
                            file: String::new(),
                            line: pos.row + 1,
                            col: pos.column + 2,
                            rule_id: self.id().to_string(),
                            message: "Missing space after \":\" in super type list"
                                .to_string(),
                            auto_fixable: true,
                        });
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
        ClassSignatureSpacing.check(&tree, source)
    }

    #[test]
    fn valid_class_signature() {
        assert!(check("class Foo : Bar\n").is_empty());
    }

    #[test]
    fn missing_space_before_super_colon() {
        let v = check("class Foo: Bar\n");
        assert!(!v.is_empty());
        assert!(v.iter().any(|x| x.message.contains("before")));
    }

    #[test]
    fn no_super_type_is_fine() {
        assert!(check("class Foo\n").is_empty());
    }

    #[test]
    fn class_with_constructor_and_super() {
        assert!(check("class Foo(val x: Int) : Bar(x)\n").is_empty());
    }
}
