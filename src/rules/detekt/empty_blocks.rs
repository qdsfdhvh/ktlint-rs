//! detekt empty-blocks rules — flag empty code blocks.
//!
//! 14 rules, all active by default, no type resolution needed.

use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

pub struct EmptyFunctionBlock;
pub struct EmptyClassBlock;
pub struct EmptyInitBlock;
pub struct EmptyDefaultConstructor;
pub struct EmptySecondaryConstructor;
pub struct EmptyIfBlock;
pub struct EmptyWhenBlock;
pub struct EmptyTryBlock;
pub struct EmptyCatchBlock;
pub struct EmptyFinallyBlock;
pub struct EmptyWhileBlock;
pub struct EmptyDoWhileBlock;
pub struct EmptyForBlock;
pub struct EmptyStructBlock;

fn is_empty_block(node: &tree_sitter::Node, source: &[u8]) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "{" | "}" => continue,
                "comment" | "multiline_comment" => continue,
                _ => {
                    let text = child.utf8_text(source).unwrap_or("");
                    if !text.chars().all(|c| c.is_whitespace()) {
                        return false;
                    }
                }
            }
        }
    }
    true
}

fn violation(rule_id: &str, node: &tree_sitter::Node) -> Violation {
    let pos = node.start_position();
    Violation {
        file: String::new(),
        line: pos.row + 1,
        col: pos.column + 1,
        rule_id: rule_id.into(),
        message: "Empty block detected".into(),
        auto_fixable: false,
    }
}

fn check_empty_blocks(
    rule_id: &str,
    tree: &Tree,
    source: &str,
    node_kind: &str,
    body_kinds: &[&str],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let bytes = source.as_bytes();
    walk_for_empty(
        rule_id,
        tree.root_node(),
        bytes,
        node_kind,
        body_kinds,
        &mut violations,
    );
    violations
}

fn walk_for_empty(
    rule_id: &str,
    node: tree_sitter::Node,
    bytes: &[u8],
    node_kind: &str,
    body_kinds: &[&str],
    violations: &mut Vec<Violation>,
) {
    if node.kind() == node_kind {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if body_kinds.contains(&child.kind()) {
                    if is_empty_block(&child, bytes) {
                        violations.push(violation(rule_id, &child));
                    }
                    break;
                }
            }
        }
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_for_empty(rule_id, child, bytes, node_kind, body_kinds, violations);
        }
    }
}

macro_rules! impl_empty_block_rule {
    ($name:ident, $id:literal, $node:literal, $body:expr) => {
        impl Rule for $name {
            fn id(&self) -> &'static str {
                $id
            }
            fn auto_fixable(&self) -> bool {
                false
            }
            fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
                check_empty_blocks($id, tree, source, $node, $body)
            }
        }
    };
}

impl_empty_block_rule!(
    EmptyFunctionBlock,
    "detekt:empty-blocks:EmptyFunctionBlock",
    "function_declaration",
    &["function_body"]
);
impl_empty_block_rule!(
    EmptyClassBlock,
    "detekt:empty-blocks:EmptyClassBlock",
    "class_declaration",
    &["class_body"]
);
impl_empty_block_rule!(
    EmptyInitBlock,
    "detekt:empty-blocks:EmptyInitBlock",
    "init_block",
    &["function_body"]
);
impl_empty_block_rule!(
    EmptyIfBlock,
    "detekt:empty-blocks:EmptyIfBlock",
    "if_expression",
    &["control_structure_body"]
);
impl_empty_block_rule!(
    EmptyWhenBlock,
    "detekt:empty-blocks:EmptyWhenBlock",
    "when_entry",
    &["control_structure_body"]
);
impl_empty_block_rule!(
    EmptyWhileBlock,
    "detekt:empty-blocks:EmptyWhileBlock",
    "while_statement",
    &["control_structure_body"]
);
impl_empty_block_rule!(
    EmptyDoWhileBlock,
    "detekt:empty-blocks:EmptyDoWhileBlock",
    "while_statement",
    &["control_structure_body"]
);
impl_empty_block_rule!(
    EmptyForBlock,
    "detekt:empty-blocks:EmptyForBlock",
    "for_statement",
    &["control_structure_body"]
);
impl_empty_block_rule!(
    EmptyFinallyBlock,
    "detekt:empty-blocks:EmptyFinallyBlock",
    "finally_block",
    &["statements"]
);

// try/catch require special handling (their AST differs from if/for/while)
fn walk_try_nodes(
    node: tree_sitter::Node,
    bytes: &[u8],
    violations: &mut Vec<Violation>,
    try_id: &str,
    catch_id: &str,
) {
    if node.kind() == "try_expression" {
        // Check try body: look for statements child
        if !try_id.is_empty() {
            let mut try_empty = true;
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "statements" && !is_empty_block(&child, bytes) {
                        try_empty = false;
                    }
                }
            }
            if try_empty {
                violations.push(violation(try_id, &node));
            }
        }
        // Check catch blocks
        if !catch_id.is_empty() {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "catch_block" {
                        let mut catch_empty = true;
                        for j in 0..child.child_count() {
                            if let Some(cc) = child.child(j) {
                                if cc.kind() == "statements" && !is_empty_block(&cc, bytes) {
                                    catch_empty = false;
                                }
                            }
                        }
                        if catch_empty {
                            violations.push(violation(catch_id, &child));
                        }
                    }
                }
            }
        }
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_try_nodes(child, bytes, violations, try_id, catch_id);
        }
    }
}

impl Rule for EmptyTryBlock {
    fn id(&self) -> &'static str {
        "detekt:empty-blocks:EmptyTryBlock"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        walk_try_nodes(tree.root_node(), bytes, &mut violations, self.id(), "");
        violations
    }
}

impl Rule for EmptyCatchBlock {
    fn id(&self) -> &'static str {
        "detekt:empty-blocks:EmptyCatchBlock"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        walk_try_nodes(tree.root_node(), bytes, &mut violations, "", self.id());
        violations
    }
}

impl Rule for EmptyDefaultConstructor {
    fn id(&self) -> &'static str {
        "detekt:empty-blocks:EmptyDefaultConstructor"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        let mut cursor = tree.root_node().walk();
        for child in tree.root_node().children(&mut cursor) {
            if child.kind() == "class_declaration" {
                for i in 0..child.child_count() {
                    if let Some(c) = child.child(i) {
                        if c.kind() == "constructor_invocation"
                            || c.kind() == "secondary_constructor"
                        {
                            for j in 0..c.child_count() {
                                if let Some(body) = c.child(j) {
                                    if body.kind() == "function_body"
                                        && is_empty_block(&body, bytes)
                                    {
                                        violations.push(violation(self.id(), &body));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        violations
    }
}

impl Rule for EmptySecondaryConstructor {
    fn id(&self) -> &'static str {
        "detekt:empty-blocks:EmptySecondaryConstructor"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        EmptyDefaultConstructor.check(tree, source)
    }
}

impl Rule for EmptyStructBlock {
    fn id(&self) -> &'static str {
        "detekt:empty-blocks:EmptyStructBlock"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, _source: &str) -> Vec<Violation> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check_rule(rule: &dyn Rule, src: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(src);
        rule.check(&tree, src)
    }

    #[test]
    fn empty_function() {
        let violations = check_rule(&EmptyFunctionBlock, "fun foo() {}");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn non_empty_function() {
        let violations = check_rule(&EmptyFunctionBlock, "fun foo() { println(\"hi\") }");
        assert!(violations.is_empty());
    }

    #[test]
    fn empty_class() {
        let violations = check_rule(&EmptyClassBlock, "class Foo {}");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn empty_if() {
        let violations = check_rule(&EmptyIfBlock, "fun f() { if (x) {} }");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn empty_try() {
        let violations = check_rule(&EmptyTryBlock, "fun f() { try {} catch(e: Exception) {} }");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn empty_catch() {
        let violations = check_rule(
            &EmptyCatchBlock,
            "fun f() { try { x() } catch(e: Exception) {} }",
        );
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn empty_for() {
        let violations = check_rule(&EmptyForBlock, "fun f() { for (i in 0..10) {} }");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn function_with_comment_still_empty() {
        let violations = check_rule(&EmptyFunctionBlock, "fun foo() { /* TODO */ }");
        assert_eq!(violations.len(), 1);
    }
}
