//! detekt complexity rules — measure code complexity via CST traversal.
//! All L0 (no type resolution required).

use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

// ── LongMethod: flag functions with too many lines ──
pub struct LongMethod {
    threshold: usize,
}
impl LongMethod {
    pub fn new() -> Self { Self { threshold: 60 } }
}
impl Rule for LongMethod {
    fn id(&self) -> &'static str { "detekt:complexity:LongMethod" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        walk_fn_bodies(tree.root_node(), &mut violations, &lines, self.threshold);
        violations
    }
}

fn walk_fn_bodies(node: tree_sitter::Node, violations: &mut Vec<Violation>, lines: &[&str], threshold: usize) {
    if node.kind() == "function_body" {
        let start = node.start_position().row;
        let end = node.end_position().row;
        let len = end.saturating_sub(start) + 1;
        if len > threshold {
            violations.push(Violation {
                file: String::new(), line: start + 1, col: 1,
                rule_id: "detekt:complexity:LongMethod".into(),
                message: format!("Function body has {} lines, exceeding threshold of {}", len, threshold),
                auto_fixable: false,
            });
        }
    }
    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) { walk_fn_bodies(c, violations, lines, threshold); }
    }
}

// ── LongParameterList: flag functions with too many parameters ──
pub struct LongParameterList {
    threshold: usize,
}
impl LongParameterList {
    pub fn new() -> Self { Self { threshold: 6 } }
}
impl Rule for LongParameterList {
    fn id(&self) -> &'static str { "detekt:complexity:LongParameterList" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, _source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_params(tree.root_node(), &mut v, self.threshold);
        v
    }
}

fn walk_params(node: tree_sitter::Node, violations: &mut Vec<Violation>, threshold: usize) {
    if node.kind() == "function_declaration" {
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                if c.kind() == "function_value_parameters" {
                    // Count parameter children (not parens)
                    let count = (0..c.child_count())
                        .filter_map(|j| c.child(j))
                        .filter(|cc| cc.kind() != "(" && cc.kind() != ")").count();
                    if count > threshold {
                        let pos = node.start_position();
                        violations.push(Violation {
                            file: String::new(), line: pos.row + 1, col: pos.column + 1,
                            rule_id: "detekt:complexity:LongParameterList".into(),
                            message: format!("Function has {} parameters, exceeding threshold of {}", count, threshold),
                            auto_fixable: false,
                        });
                    }
                    break;
                }
            }
        }
    }
    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) { walk_params(c, violations, threshold); }
    }
}

// ── NestedBlockDepth: flag deeply nested blocks ──
pub struct NestedBlockDepth {
    threshold: usize,
}
impl NestedBlockDepth {
    pub fn new() -> Self { Self { threshold: 4 } }
}
impl Rule for NestedBlockDepth {
    fn id(&self) -> &'static str { "detekt:complexity:NestedBlockDepth" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, _source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_nesting(tree.root_node(), 0, &mut v, self.threshold);
        v
    }
}

fn walk_nesting(node: tree_sitter::Node, depth: usize, violations: &mut Vec<Violation>, threshold: usize) {
    let kind = node.kind();
    let opens_block = matches!(kind,
        "function_body" | "control_structure_body" | "class_body" |
        "if_expression" | "when_expression" | "when_entry" | "while_statement" |
        "for_statement" | "do_while_statement" | "try_expression" | "catch_block" | "finally_block" |
        "lambda_literal" | "anonymous_function"
    );

    let new_depth = if opens_block { depth + 1 } else { depth };

    if new_depth > threshold {
        let pos = node.start_position();
        violations.push(Violation {
            file: String::new(), line: pos.row + 1, col: pos.column + 1,
            rule_id: "detekt:complexity:NestedBlockDepth".into(),
            message: format!("Nesting depth {} exceeds threshold of {}", new_depth, threshold),
            auto_fixable: false,
        });
    }

    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) {
            walk_nesting(c, new_depth, violations, threshold);
        }
    }
}

// ── LargeClass: flag classes with too many methods ──
pub struct LargeClass {
    threshold: usize,
}
impl LargeClass {
    pub fn new() -> Self { Self { threshold: 30 } }
}
impl Rule for LargeClass {
    fn id(&self) -> &'static str { "detekt:complexity:LargeClass" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, _source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_classes(tree.root_node(), &mut v, self.threshold);
        v
    }
}

fn walk_classes(node: tree_sitter::Node, violations: &mut Vec<Violation>, threshold: usize) {
    if node.kind() == "class_body" {
        let method_count = (0..node.child_count())
            .filter_map(|i| node.child(i))
            .filter(|c| c.kind() == "function_declaration")
            .count();
        if method_count > threshold {
            let pos = node.start_position();
            violations.push(Violation {
                file: String::new(), line: pos.row + 1, col: pos.column + 1,
                rule_id: "detekt:complexity:LargeClass".into(),
                message: format!("Class has {} methods, exceeding threshold of {}", method_count, threshold),
                auto_fixable: false,
            });
        }
    }
    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) { walk_classes(c, violations, threshold); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn check(rule: &dyn Rule, s: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(s);
        rule.check(&tree, s)
    }

    #[test] fn long_method_short() { assert!(check(&LongMethod::new(), "fun f() { println() }\n").is_empty()); }
    #[test] fn long_param_short() { assert!(check(&LongParameterList::new(), "fun f(a: Int)\n").is_empty()); }
    #[test] fn long_param_exceeds() {
        assert!(!check(&LongParameterList { threshold: 2 }, "fun f(a: Int, b: Int, c: Int)\n").is_empty());
    }
    #[test] fn nested_shallow() { assert!(check(&NestedBlockDepth::new(), "fun f() { if(x){ println() } }\n").is_empty()); }
}
