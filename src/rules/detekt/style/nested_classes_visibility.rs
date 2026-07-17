//! detekt:style:NestedClassesVisibility — nested classes should not have
//! broader explicit visibility than their enclosing class.
use crate::rules::{Rule, Violation};
use tree_sitter::Node;

pub struct NestedClassesVisibility;

/// Visibility rank: higher = more permissive.
fn rank(vis: &str) -> u8 {
    match vis {
        "private" => 0,
        "protected" => 1,
        "internal" => 2,
        "public" => 3,
        _ => 3, // implicit = public
    }
}

/// Extract the explicit visibility keyword of a class node, if any.
fn explicit_visibility(node: &Node, bytes: &[u8]) -> Option<&'static str> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "modifiers" {
                let text = child.utf8_text(bytes).unwrap_or("");
                if text.contains("private") {
                    return Some("private");
                }
                if text.contains("protected") {
                    return Some("protected");
                }
                if text.contains("internal") {
                    return Some("internal");
                }
                if text.contains("public") {
                    return Some("public");
                }
            }
        }
    }
    None
}

impl Rule for NestedClassesVisibility {
    fn id(&self) -> &'static str {
        "detekt:style:NestedClassesVisibility"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        walk(tree.root_node(), bytes, None, &mut v);
        v
    }
}

/// Walk the CST, carrying the effective visibility rank of the enclosing class.
fn walk(node: Node, bytes: &[u8], parent_rank: Option<u8>, v: &mut Vec<Violation>) {
    let mut next_parent = parent_rank;
    if matches!(
        node.kind(),
        "class_declaration" | "object_declaration" | "enum_class"
    ) {
        let explicit = explicit_visibility(&node, bytes);
        let own_rank = explicit.map(rank).unwrap_or(3);
        // Only flag when the enclosing class restricts visibility (internal/private)
        if let Some(pr) = parent_rank {
            if pr <= 2 {
                if let Some(evis) = explicit {
                    if rank(evis) > pr {
                        let pos = node.start_position();
                        v.push(Violation {
                            file: String::new(),
                            line: pos.row + 1,
                            col: pos.column + 1,
                            rule_id: "detekt:style:NestedClassesVisibility".into(),
                            message: format!(
                                "Nested class has '{}' visibility but outer class is more restrictive",
                                evis
                            ),
                            auto_fixable: false,
                        });
                    }
                }
            }
        }
        // The effective visibility of children is capped by the parent
        let effective = match parent_rank {
            Some(pr) => own_rank.min(pr),
            None => own_rank,
        };
        next_parent = Some(effective);
    }
    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) {
            walk(c, bytes, next_parent, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        NestedClassesVisibility.check(&KotlinParser::new().parse(s), s)
    }
    #[test]
    fn public_nested_in_internal_bad() {
        assert!(!c("internal class Outer {\n    public class Nested\n}\n").is_empty());
    }
    #[test]
    fn internal_nested_in_private_bad() {
        assert!(!c("private class Outer {\n    internal class Nested\n}\n").is_empty());
    }
    #[test]
    fn implicit_nested_ok() {
        assert!(c("internal class Outer {\n    class Nested\n}\n").is_empty());
    }
    #[test]
    fn public_outer_ok() {
        assert!(c("class Outer {\n    public class Nested\n}\n").is_empty());
    }
    #[test]
    fn same_visibility_ok() {
        assert!(c("internal class Outer {\n    internal class Nested\n}\n").is_empty());
    }
}
