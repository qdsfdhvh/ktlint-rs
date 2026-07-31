//! Spacing rules for Kotlin function types and extension receivers.

use crate::rules::{Rule, Violation};

pub struct FunctionTypeModifierSpacing;
pub struct FunctionTypeReferenceSpacing;

impl Rule for FunctionTypeModifierSpacing {
    fn id(&self) -> &'static str {
        "standard:function-type-modifier-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        walk(tree.root_node(), &mut |node| {
            if node.kind() != "type_modifiers" {
                return;
            }
            let Some(function_type) = node.next_named_sibling() else {
                return;
            };
            if function_type.kind() != "function_type" {
                return;
            }
            let gap = &source[node.end_byte()..function_type.start_byte()];
            if gap == " " {
                return;
            }
            violations.push(violation_at_offset(
                source,
                function_type.start_byte(),
                self.id(),
                "Expected a single space between the modifier list and the function type",
            ));
        });
        violations
    }
}

impl Rule for FunctionTypeReferenceSpacing {
    fn id(&self) -> &'static str {
        "standard:function-type-reference-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        walk(tree.root_node(), &mut |node| {
            if node.kind() != "function_declaration" {
                return;
            }
            let text = &source[node.byte_range()];
            let header_end = text.find('(').unwrap_or(text.len());
            let header = &text[..header_end];
            let whitespace = header
                .find(" ?.")
                .or_else(|| header.find(" ."))
                .map(|index| node.start_byte() + index);
            if let Some(offset) = whitespace {
                violations.push(violation_at_offset(
                    source,
                    offset,
                    self.id(),
                    "Unexpected whitespace",
                ));
            }
        });
        violations
    }
}

fn violation_at_offset(source: &str, offset: usize, rule_id: &str, message: &str) -> Violation {
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let col = source[line_start..offset].chars().count() + 1;
    Violation {
        file: String::new(),
        line,
        col,
        rule_id: rule_id.into(),
        message: message.into(),
        auto_fixable: true,
    }
}

fn walk(root: tree_sitter::Node, visit: &mut impl FnMut(tree_sitter::Node)) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        visit(node);
        for index in (0..node.child_count()).rev() {
            if let Some(child) = node.child(index) {
                stack.push(child);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(rule: &dyn Rule, source: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(source);
        rule.check(&tree, source)
    }

    #[test]
    fn function_type_modifier_requires_exactly_one_space() {
        let violation = check(
            &FunctionTypeModifierSpacing,
            "val foo: suspend() -> Unit = {}\n",
        );
        assert_eq!(violation.len(), 1);
        assert_eq!((violation[0].line, violation[0].col), (1, 17));
        assert_eq!(
            violation[0].message,
            "Expected a single space between the modifier list and the function type"
        );
        assert!(check(
            &FunctionTypeModifierSpacing,
            "val foo: suspend () -> Unit = {}\n"
        )
        .is_empty());
    }

    #[test]
    fn function_type_modifier_handles_newline_gap() {
        let violation = check(
            &FunctionTypeModifierSpacing,
            "val foo: suspend\n         () -> Unit = {}\n",
        );
        assert_eq!((violation[0].line, violation[0].col), (2, 10));
    }

    #[test]
    fn extension_receiver_must_touch_dot_or_safe_dot() {
        let dot = check(&FunctionTypeReferenceSpacing, "fun String .foo() = Unit\n");
        assert_eq!((dot[0].line, dot[0].col), (1, 11));
        let safe = check(&FunctionTypeReferenceSpacing, "fun String ?.foo() = Unit\n");
        assert_eq!((safe[0].line, safe[0].col), (1, 11));
        assert!(check(&FunctionTypeReferenceSpacing, "fun String?.foo() = Unit\n").is_empty());
    }
}
