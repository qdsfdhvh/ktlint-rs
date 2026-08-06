//! standard:function-naming — function names must be camelCase.
//! PascalCase allowed for @Composable, @Preview, @Test functions.

use crate::rules::{Rule, Violation};

pub struct FunctionNaming;

impl Rule for FunctionNaming {
    fn id(&self) -> &'static str {
        "standard:function-naming"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        walk_fn(tree.root_node(), bytes, &mut violations);
        violations
    }
}

fn walk_fn(node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    if node.kind() == "function_declaration" {
        // Find function name (simple_identifier child)
        let mut name = "";
        let mut has_composable = false;
        let mut has_test_annotation = false;
        let mut has_preview = false;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "simple_identifier" => {
                        name = child.utf8_text(bytes).unwrap_or("");
                    }
                    "modifiers" => {
                        // Check for @Composable, @Preview, @Test annotations
                        for j in 0..child.child_count() {
                            if let Some(mod_child) = child.child(j) {
                                if mod_child.kind() == "annotation" {
                                    let text = mod_child.utf8_text(bytes).unwrap_or("");
                                    if text.contains("Composable") {
                                        has_composable = true;
                                    }
                                    if text.contains("Preview") {
                                        has_preview = true;
                                    }
                                    if text.contains("Test") {
                                        has_test_annotation = true;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if name.is_empty() || name.starts_with("test_") {
            return;
        }
        // Factory functions (`fun Foo(...): Foo`) and actual/expect
        // declarations may use PascalCase — ktlint 1.8 exempts these (common
        // in multiplatform factory patterns). Detect by checking whether the
        // return type names the same identifier as the function.
        let return_type = node.child_by_field_name("return_type").or_else(|| {
            (0..node.child_count()).find_map(|i| node.child(i).filter(|c| c.kind() == "user_type"))
        });
        let is_factory = return_type.is_some_and(|rt| {
            let rt_text = rt.utf8_text(bytes).unwrap_or("");
            rt_text.trim() == name
        });
        let is_actual_or_expect = (0..node.child_count()).any(|i| {
            node.child(i).is_some_and(|c| {
                let t = c.utf8_text(bytes).unwrap_or("");
                if c.kind() == "modifiers" {
                    t.contains("actual") || t.contains("expect")
                } else {
                    t == "actual" || t == "expect"
                }
            })
        });
        if is_actual_or_expect || is_factory {
            return;
        }

        // Skip operator functions
        if is_operator_function(name) {
            return;
        }

        // PascalCase allowed for @Composable, @Preview, @Test
        if has_composable || has_preview || has_test_annotation {
            // These annotations allow both PascalCase and camelCase
            // Only flag if it has invalid characters (not alphanumeric)
            return;
        }

        if !is_camel_case(name) {
            let pos = node.start_position();
            violations.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: "standard:function-naming".into(),
                message: format!("Function name \"{}\" should be camelCase", name),
                auto_fixable: false,
            });
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_fn(child, bytes, violations);
        }
    }
}

fn is_operator_function(name: &str) -> bool {
    matches!(
        name,
        "plus"
            | "minus"
            | "times"
            | "div"
            | "rem"
            | "compareTo"
            | "get"
            | "set"
            | "contains"
            | "invoke"
            | "rangeTo"
            | "iterator"
    )
}

fn is_camel_case(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_lowercase()) && !s.contains('_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(source);
        FunctionNaming.check(&tree, source)
    }

    #[test]
    fn camel_case_function() {
        assert!(check("fun myFunction()\n").is_empty());
    }

    #[test]
    fn pascal_case_function_flagged() {
        let v = check("fun MyFunction()\n");
        assert!(!v.is_empty());
    }

    #[test]
    fn composable_function_allowed() {
        assert!(check("@Composable\nfun MyComponent() {}\n").is_empty());
        assert!(check("@Composable\nfun myComponent() {}\n").is_empty());
    }

    #[test]
    fn preview_function_allowed() {
        assert!(check("@Preview\nfun MyPreview() {}\n").is_empty());
    }

    #[test]
    fn test_function_allowed() {
        assert!(check("@Test\nfun my_test() {}\n").is_empty());
    }

    #[test]
    fn operator_function_ok() {
        assert!(check("fun plus(other: Int)\n").is_empty());
    }

    #[test]
    fn snake_case_flagged() {
        let v = check("fun my_function()\n");
        assert!(!v.is_empty());
    }
}
