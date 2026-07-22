//! standard:colon-spacing — space before/after `:` depending on context.
//!
//! - Type annotations: no space before `:`, space after `:`
//!   `val x: Int`, `fun foo(): String`
//! - Super type list: space before and after `:`
//!   `class Foo : Base`
//! - `::` (method reference): no spaces

use crate::rules::{Rule, Violation};

pub struct ColonSpacing;

impl Rule for ColonSpacing {
    fn id(&self) -> &'static str {
        "standard:colon-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl ColonSpacing {
    fn walk(&self, node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        if node.kind() == ":" {
            self.check_colon(&node, bytes, violations);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, bytes, violations);
            }
        }
    }

    fn is_in_annotation_context(&self, node: &tree_sitter::Node) -> bool {
        // Check if this `:` is inside an annotation (e.g., @get:Rule, @file:Suppress)
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.kind() == "annotation" {
                return true;
            }
            current = parent.parent();
        }
        false
    }

    fn is_in_type_context(&self, node: &tree_sitter::Node) -> bool {
        // Check parent to determine context
        if let Some(parent) = node.parent() {
            let parent_kind = parent.kind();
            matches!(
                parent_kind,
                "parameter"
                    | "catch_block"
                    | "class_parameter"
                    | "function_declaration"
                    | "property_declaration"
                    | "variable_declaration"
                    | "type_annotation"
                    | "when_entry"
                    | "lambda_parameter"
            )
        } else {
            false
        }
    }

    fn check_colon(&self, node: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let pos = node.start_position();
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();

        // Skip `::` (double colon — method reference)
        if end_byte < bytes.len() && bytes[end_byte] == b':' {
            return;
        }
        if start_byte > 0 && bytes[start_byte - 1] == b':' {
            return;
        }

        // Skip annotation target colons (e.g., @get:Rule, @file:Suppress)
        if self.is_in_annotation_context(node) {
            return;
        }

        let is_type_context = self.is_in_type_context(node);

        if is_type_context {
            // Type annotation: no space before, space after
            if start_byte > 0 && bytes[start_byte - 1] == b' ' {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column,
                    rule_id: self.id().to_string(),
                    message: "Unexpected space before \":\" in type annotation".to_string(),
                    auto_fixable: true,
                });
            }
            if end_byte < bytes.len() && bytes[end_byte] != b' ' && bytes[end_byte] != b'\n' {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 2,
                    rule_id: self.id().to_string(),
                    message: "Missing space after \":\" in type annotation".to_string(),
                    auto_fixable: true,
                });
            }
        } else {
            // Super type list, etc.: space before and after
            if start_byte > 0 && bytes[start_byte - 1] != b' ' && bytes[start_byte - 1] != b'\n' {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column,
                    rule_id: self.id().to_string(),
                    message: "Missing space before \":\"".to_string(),
                    auto_fixable: true,
                });
            }
            if end_byte < bytes.len() && bytes[end_byte] != b' ' && bytes[end_byte] != b'\n' {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 2,
                    rule_id: self.id().to_string(),
                    message: "Missing space after \":\"".to_string(),
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
        ColonSpacing.check(&tree, source)
    }

    #[test]
    fn type_annotation_colon() {
        assert!(check("val x: Int = 1\n").is_empty());
    }

    #[test]
    fn space_before_type_annotation_colon() {
        let v = check("val x : Int = 1\n");
        assert!(!v.is_empty());
        assert!(v.iter().any(|x| x.message.contains("type annotation")));
    }

    #[test]
    fn super_type_colon() {
        assert!(check("class Foo : Base\n").is_empty());
    }

    #[test]
    fn function_return_type() {
        assert!(check("fun foo(): String\n").is_empty());
    }
}
