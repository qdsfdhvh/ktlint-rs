//! ktlint 1.8 enum entry casing and deprecated compatibility provider.

use crate::rules::{Rule, Violation};

pub struct EnumEntryNameCase;
pub struct DiscouragedCommentLocation;

impl Rule for EnumEntryNameCase {
    fn id(&self) -> &'static str {
        "standard:enum-entry-name-case"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        walk(tree.root_node(), &mut |node| {
            if node.kind() != "enum_entry" {
                return;
            }
            let Some(identifier) = find_identifier(node) else {
                return;
            };
            let name = &source[identifier.byte_range()];
            if is_upper_snake(name) || is_upper_camel(name) {
                return;
            }
            let position = identifier.start_position();
            violations.push(Violation {
                file: String::new(),
                line: position.row + 1,
                col: position.column + 1,
                rule_id: self.id().into(),
                message: "Enum entry name should be uppercase underscore-separated names like \"ENUM_ENTRY\" or upper camel-case like \"EnumEntry\"".into(),
                auto_fixable: false,
            });
        });
        violations
    }
}

/// ktlint 1.8 keeps this provider for compatibility, but its implementation is
/// intentionally empty and deprecated for removal in ktlint 2.0.
impl Rule for DiscouragedCommentLocation {
    fn id(&self) -> &'static str {
        "standard:discouraged-comment-location"
    }

    fn check(&self, _tree: &tree_sitter::Tree, _source: &str) -> Vec<Violation> {
        Vec::new()
    }
}

fn find_identifier(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if matches!(current.kind(), "simple_identifier" | "identifier") {
            return Some(current);
        }
        for index in (0..current.child_count()).rev() {
            if let Some(child) = current.child(index) {
                stack.push(child);
            }
        }
    }
    None
}

fn is_upper_snake(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('_')
        && !name.ends_with('_')
        && !name.contains("__")
        && name
            .chars()
            .all(|character| character == '_' || character.is_uppercase() || character.is_numeric())
}

fn is_upper_camel(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
        && !name.contains('_')
        && name.chars().all(char::is_alphanumeric)
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
    fn accepts_upper_snake_upper_camel_and_diacritics() {
        let source = "enum class E { FOO, FooBar, ŸÈŚ_THÎS }\n";
        assert!(check(&EnumEntryNameCase, source).is_empty());
    }

    #[test]
    fn rejects_lowercase_mixed_underscore_and_leading_underscore() {
        for name in ["foo", "Foo_Bar", "_FOO"] {
            let source = format!("enum class E {{ {name} }}\n");
            let violations = check(&EnumEntryNameCase, &source);
            assert_eq!(violations.len(), 1, "{name}");
            assert!(!violations[0].auto_fixable);
            assert_eq!(
                violations[0].message,
                "Enum entry name should be uppercase underscore-separated names like \"ENUM_ENTRY\" or upper camel-case like \"EnumEntry\""
            );
        }
    }

    #[test]
    fn discouraged_comment_location_is_a_noop_in_ktlint_1_8() {
        let source = "enum class E { // deliberately awkward\n    FOO\n}\n";
        assert!(check(&DiscouragedCommentLocation, source).is_empty());
    }
}
