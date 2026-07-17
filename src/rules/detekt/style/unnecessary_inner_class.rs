//! detekt:style:UnnecessaryInnerClass — inner classes that don't use
//! outer class members can be regular nested classes.
use crate::rules::{Rule, Violation};
use tree_sitter::Node;

pub struct UnnecessaryInnerClass;

/// Check if a class has the `inner` modifier.
fn is_inner(node: &Node, bytes: &[u8]) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "modifiers" {
                return child.utf8_text(bytes).unwrap_or("").contains("inner");
            }
        }
    }
    false
}

/// Collect simple_identifier names within `node`.
fn collect_ids(node: &Node, bytes: &[u8], names: &mut Vec<String>) {
    if node.kind() == "simple_identifier" || node.kind() == "identifier" {
        if let Ok(n) = node.utf8_text(bytes) {
            names.push(n.to_string());
        }
    }
    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) {
            collect_ids(&c, bytes, names);
        }
    }
}

/// Collect member names of the outer class declaration: primary-constructor
/// parameters plus class-body properties/functions (excluding the inner class
/// itself).
fn collect_outer_members(outer_decl: &Node, bytes: &[u8], skip: &Node) -> Vec<String> {
    let mut m = Vec::new();
    for i in 0..outer_decl.child_count() {
        if let Some(child) = outer_decl.child(i) {
            match child.kind() {
                "primary_constructor" => {
                    collect_ids(&child, bytes, &mut m);
                }
                "class_body" => {
                    for j in 0..child.child_count() {
                        if let Some(member) = child.child(j) {
                            if member.id() == skip.id() {
                                continue;
                            }
                            if matches!(
                                member.kind(),
                                "property_declaration" | "function_declaration"
                            ) {
                                collect_ids(&member, bytes, &mut m);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    m
}

/// Check if `node` contains a labeled `this@Outer` expression.
fn has_qualified_this(node: &Node, bytes: &[u8]) -> bool {
    if node.kind() == "this_expression" {
        let text = node.utf8_text(bytes).unwrap_or("");
        if text.contains('@') {
            return true;
        }
    }
    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) {
            if has_qualified_this(&c, bytes) {
                return true;
            }
        }
    }
    false
}

impl Rule for UnnecessaryInnerClass {
    fn id(&self) -> &'static str {
        "detekt:style:UnnecessaryInnerClass"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        walk(tree.root_node(), bytes, &mut v);
        v
    }
}

fn walk(node: Node, bytes: &[u8], v: &mut Vec<Violation>) {
    if node.kind() == "class_declaration" && is_inner(&node, bytes) {
        if let Some(outer_decl) = find_outer_class_decl(&node) {
            let outer_members = collect_outer_members(&outer_decl, bytes, &node);

            // Identifiers referenced inside the inner class
            let mut refs = Vec::new();
            collect_ids(&node, bytes, &mut refs);

            let uses_outer =
                refs.iter().any(|r| outer_members.contains(r)) || has_qualified_this(&node, bytes);

            if !uses_outer {
                let pos = node.start_position();
                v.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 1,
                    rule_id: "detekt:style:UnnecessaryInnerClass".into(),
                    message: "Inner class does not use outer class members — remove 'inner'".into(),
                    auto_fixable: false,
                });
            }
        }
    }
    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) {
            walk(c, bytes, v);
        }
    }
}

/// Find the enclosing class_declaration of an inner class node.
fn find_outer_class_decl<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    let mut current = node.parent()?;
    loop {
        if current.kind() == "class_declaration" {
            return Some(current);
        }
        current = current.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        UnnecessaryInnerClass.check(&KotlinParser::new().parse(s), s)
    }
    #[test]
    fn inner_uses_outer_ok() {
        assert!(
            c("class Outer { val x = 1\n    inner class Inner { fun f() = x }\n}\n").is_empty()
        );
    }
    #[test]
    fn inner_uses_ctor_property_ok() {
        assert!(
            c("class Outer(private val x: Int) {\n    inner class Inner { fun f() = x }\n}\n")
                .is_empty()
        );
    }
    #[test]
    fn inner_no_use_bad() {
        assert!(
            !c("class Outer { val x = 1\n    inner class Inner { fun f() = 2 }\n}\n").is_empty()
        );
    }
    #[test]
    fn inner_with_qualified_this_ok() {
        assert!(c(
            "class Outer { val x = 1\n    inner class Inner { fun f() = this@Outer.x }\n}\n"
        )
        .is_empty());
    }
}
