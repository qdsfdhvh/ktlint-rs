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

    fn is_annotation_use_site_target(&self, start_byte: usize, bytes: &[u8]) -> bool {
        let mut cursor = start_byte;
        while cursor > 0 {
            let byte = bytes[cursor - 1];
            if byte.is_ascii_alphanumeric() || byte == b'_' {
                cursor -= 1;
            } else {
                break;
            }
        }
        cursor > 0 && bytes[cursor - 1] == b'@'
    }

    fn requires_space_before(&self, node: &tree_sitter::Node) -> bool {
        let Some(parent) = node.parent() else {
            return false;
        };
        if matches!(
            parent.kind(),
            "class_declaration"
                | "object_declaration"
                | "object_literal"
                | "delegation_specifier"
                | "delegation_specifiers"
        ) {
            return true;
        }
        let mut current = Some(parent);
        while let Some(ancestor) = current {
            if matches!(
                ancestor.kind(),
                "type_constraint" | "type_parameter" | "type_parameters"
            ) {
                return true;
            }
            if ancestor.kind() == "class_declaration" {
                break;
            }
            if matches!(
                ancestor.kind(),
                "function_declaration"
                    | "property_declaration"
                    | "value_parameter"
                    | "class_parameter"
            ) {
                break;
            }
            current = ancestor.parent();
        }
        false
    }

    fn is_class_or_constructor_context(node: &tree_sitter::Node) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                "secondary_constructor" | "class_declaration" | "object_declaration" => {
                    return true
                }
                "function_declaration" | "property_declaration" => return false,
                _ => current = parent.parent(),
            }
        }
        false
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

        // Some tree-sitter Kotlin grammar versions do not put the target colon below
        // an `annotation` node, so also recognize the lexical `@target:` prefix.
        if self.is_in_annotation_context(node)
            || self.is_annotation_use_site_target(start_byte, bytes)
        {
            return;
        }

        let requires_space_before = self.requires_space_before(node);
        let line_start = bytes[..start_byte]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        let prefix = &bytes[line_start..start_byte];
        let last_open = prefix.iter().rposition(|byte| *byte == b'<');
        let last_close = prefix.iter().rposition(|byte| *byte == b'>');
        let inside_type_parameters = last_open.is_some() && last_open > last_close;
        let trimmed_prefix = prefix.strip_suffix(b" ").unwrap_or(prefix);
        let open_parens = trimmed_prefix.iter().filter(|byte| **byte == b'(').count();
        let close_parens = trimmed_prefix.iter().filter(|byte| **byte == b')').count();
        let follows_closed_header = trimmed_prefix.ends_with(b")")
            && close_parens >= open_parens
            && Self::is_class_or_constructor_context(node);
        let before_colon = &bytes[..start_byte];
        let last_class = before_colon
            .windows(6)
            .rposition(|window| window == b"class ");
        let last_fun = before_colon
            .windows(4)
            .rposition(|window| window == b"fun ");
        let in_class_header = last_class.is_some()
            && last_class > last_fun
            && !before_colon[last_class.unwrap_or(0)..].contains(&b'{');
        let requires_space_before = requires_space_before
            || inside_type_parameters
            || follows_closed_header
            || (trimmed_prefix.ends_with(b")") && in_class_header);

        if requires_space_before {
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
        } else if start_byte > 0 && bytes[start_byte - 1] == b' ' {
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
                message: "Missing space after \":\"".to_string(),
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

    #[test]
    fn annotation_use_site_target_colons() {
        assert!(check("@file:OptIn(ExperimentalApi::class)\nclass Foo\n").is_empty());
        assert!(check("class Foo(@get:Inject val value: String)\n").is_empty());
    }

    #[test]
    fn function_type_parameter_colons() {
        assert!(check("val callback: (value: String, count: Int) -> Unit\n").is_empty());
        assert!(check("val content: @Composable (onDismiss: () -> Unit) -> Unit\n").is_empty());
    }
}
