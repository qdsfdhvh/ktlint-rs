//! detekt complexity rules — measure code complexity via CST traversal.
//! All L0 (no type resolution required).

use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

// ── LongMethod ──
pub struct LongMethod { threshold: usize }
impl LongMethod {
    pub fn new() -> Self { Self { threshold: 60 } }
}
impl Rule for LongMethod {
    fn id(&self) -> &'static str { "detekt:complexity:LongMethod" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        walk_method_len(tree.root_node(), &mut v, &lines, self.threshold);
        v
    }
}

fn walk_method_len(n: tree_sitter::Node, v: &mut Vec<Violation>, lines: &[&str], t: usize) {
    if n.kind() == "function_body" {
        let len = n.end_position().row.saturating_sub(n.start_position().row) + 1;
        if len > t {
            let pos = n.start_position();
            v.push(Violation {
                file: String::new(), line: pos.row + 1, col: pos.column + 1,
                rule_id: "detekt:complexity:LongMethod".into(),
                message: format!("Function body has {} lines, exceeding threshold of {}", len, t),
                auto_fixable: false,
            });
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            walk_method_len(c, v, lines, t);
        }
    }
}

// ── LongParameterList ──
pub struct LongParameterList { threshold: usize }
impl LongParameterList {
    pub fn new() -> Self { Self { threshold: 6 } }
}
impl Rule for LongParameterList {
    fn id(&self) -> &'static str { "detekt:complexity:LongParameterList" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_param_count(tree.root_node(), &mut v, self.threshold);
        v
    }
}

fn walk_param_count(n: tree_sitter::Node, v: &mut Vec<Violation>, t: usize) {
    if n.kind() == "function_declaration" {
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i) {
                if c.kind() == "function_value_parameters" {
                    let count = (0..c.child_count())
                        .filter_map(|j| c.child(j))
                        .filter(|cc| cc.kind() != "(" && cc.kind() != ")").count();
                    if count > t {
                        let pos = n.start_position();
                        v.push(Violation {
                            file: String::new(), line: pos.row + 1, col: pos.column + 1,
                            rule_id: "detekt:complexity:LongParameterList".into(),
                            message: format!(
                                "Function has {} parameters, exceeding threshold of {}", count, t
                            ),
                            auto_fixable: false,
                        });
                    }
                    break;
                }
            }
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            walk_param_count(c, v, t);
        }
    }
}

// ── NestedBlockDepth ──
pub struct NestedBlockDepth { threshold: usize }
impl NestedBlockDepth {
    pub fn new() -> Self { Self { threshold: 4 } }
}
impl Rule for NestedBlockDepth {
    fn id(&self) -> &'static str { "detekt:complexity:NestedBlockDepth" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_depth(tree.root_node(), 0, &mut v, self.threshold);
        v
    }
}

fn walk_depth(n: tree_sitter::Node, depth: usize, v: &mut Vec<Violation>, t: usize) {
    let opens = matches!(n.kind(),
        "function_body" | "control_structure_body" | "class_body" |
        "if_expression" | "when_expression" | "when_entry" | "while_statement" |
        "for_statement" | "do_while_statement" | "try_expression" |
        "catch_block" | "finally_block" | "lambda_literal" | "anonymous_function"
    );
    let d = if opens { depth + 1 } else { depth };
    if d > t {
        let pos = n.start_position();
        v.push(Violation {
            file: String::new(), line: pos.row + 1, col: pos.column + 1,
            rule_id: "detekt:complexity:NestedBlockDepth".into(),
            message: format!("Nesting depth {} exceeds threshold of {}", d, t),
            auto_fixable: false,
        });
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { walk_depth(c, d, v, t); }
    }
}

// ── LargeClass ──
pub struct LargeClass { threshold: usize }
impl LargeClass {
    pub fn new() -> Self { Self { threshold: 30 } }
}
impl Rule for LargeClass {
    fn id(&self) -> &'static str { "detekt:complexity:LargeClass" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_class_methods(tree.root_node(), &mut v, self.threshold);
        v
    }
}

fn walk_class_methods(n: tree_sitter::Node, v: &mut Vec<Violation>, t: usize) {
    if n.kind() == "class_body" {
        let count = (0..n.child_count())
            .filter_map(|i| n.child(i))
            .filter(|c| c.kind() == "function_declaration").count();
        if count > t {
            let pos = n.start_position();
            v.push(Violation {
                file: String::new(), line: pos.row + 1, col: pos.column + 1,
                rule_id: "detekt:complexity:LargeClass".into(),
                message: format!("Class has {} methods, exceeding threshold of {}", count, t),
                auto_fixable: false,
            });
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { walk_class_methods(c, v, t); }
    }
}

// ── CyclomaticComplexMethod ──
pub struct CyclomaticComplexMethod { threshold: usize }
impl CyclomaticComplexMethod {
    pub fn new() -> Self { Self { threshold: 15 } }
}
impl Rule for CyclomaticComplexMethod {
    fn id(&self) -> &'static str { "detekt:complexity:CyclomaticComplexMethod" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_cyclo(tree.root_node(), &mut v, self.threshold);
        v
    }
}

fn walk_cyclo(n: tree_sitter::Node, v: &mut Vec<Violation>, t: usize) {
    if n.kind() == "function_body" {
        let complexity = 1 + count_branches(&n);
        if complexity > t {
            let pos = n.start_position();
            v.push(Violation {
                file: String::new(), line: pos.row + 1, col: pos.column + 1,
                rule_id: "detekt:complexity:CyclomaticComplexMethod".into(),
                message: format!("Cyclomatic complexity {} exceeds threshold of {}", complexity, t),
                auto_fixable: false,
            });
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { walk_cyclo(c, v, t); }
    }
}

fn count_branches(n: &tree_sitter::Node) -> usize {
    let kind = n.kind();
    let mut count = if matches!(kind, "if_expression" | "when_entry" | "while_statement" |
        "for_statement" | "do_while_statement" | "catch_block")
    { 1 } else { 0 };
    // AND/OR in conditions
    if kind == "conjunction_expression" || kind == "disjunction_expression" { count += 1; }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { count += count_branches(&c); }
    }
    count
}

// ── TooManyFunctions ──
pub struct TooManyFunctions { threshold: usize }
impl TooManyFunctions {
    pub fn new() -> Self { Self { threshold: 20 } }
}
impl Rule for TooManyFunctions {
    fn id(&self) -> &'static str { "detekt:complexity:TooManyFunctions" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        // Check per-file: count all function_declarations
        let count = count_fns(tree.root_node());
        if count > self.threshold {
            v.push(Violation {
                file: String::new(), line: 1, col: 1,
                rule_id: "detekt:complexity:TooManyFunctions".into(),
                message: format!("File has {} functions, exceeding threshold of {}", count, self.threshold),
                auto_fixable: false,
            });
        }
        v
    }
}

fn count_fns(n: tree_sitter::Node) -> usize {
    let mut count = if n.kind() == "function_declaration" { 1 } else { 0 };
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { count += count_fns(c); }
    }
    count
}

// ── ComplexCondition ──
pub struct ComplexCondition { threshold: usize }
impl ComplexCondition {
    pub fn new() -> Self { Self { threshold: 3 } }
}
impl Rule for ComplexCondition {
    fn id(&self) -> &'static str { "detekt:complexity:ComplexCondition" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_cond(tree.root_node(), &mut v, self.threshold);
        v
    }
}

fn walk_cond(n: tree_sitter::Node, v: &mut Vec<Violation>, t: usize) {
    if n.kind() == "if_expression" || n.kind() == "when_entry" || n.kind() == "while_statement" {
        // Count logical operators in condition
        let ops = count_ops(&n);
        if ops > t {
            let pos = n.start_position();
            v.push(Violation {
                file: String::new(), line: pos.row + 1, col: pos.column + 1,
                rule_id: "detekt:complexity:ComplexCondition".into(),
                message: format!("Condition has {} logical operators, exceeding threshold of {}", ops, t),
                auto_fixable: false,
            });
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { walk_cond(c, v, t); }
    }
}

fn count_ops(n: &tree_sitter::Node) -> usize {
    let k = n.kind();
    let mut count = if k == "conjunction_expression" || k == "disjunction_expression" { 1 } else { 0 };
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { count += count_ops(&c); }
    }
    count
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(r: &dyn Rule, s: &str) -> Vec<Violation> { r.check(&KotlinParser::new().parse(s), s) }

    #[test] fn long_method_ok() { assert!(c(&LongMethod::new(), "fun f(){println()}\n").is_empty()); }
    #[test] fn long_param_ok() { assert!(c(&LongParameterList::new(), "fun f(a:Int)\n").is_empty()); }
    #[test] fn long_param_exceeds() {
        assert!(!c(&LongParameterList{threshold:2}, "fun f(a:Int,b:Int,c:Int)\n").is_empty());
    }
    #[test] fn nested_ok() { assert!(c(&NestedBlockDepth::new(), "fun f(){if(x){println()}}\n").is_empty()); }
}
