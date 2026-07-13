//! detekt naming rules — name convention checks via CST.

use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

// ── FunctionMaxLength ──
pub struct FunctionMaxLength { max: usize }
impl FunctionMaxLength { pub fn new() -> Self { Self { max: 40 } } }
impl Rule for FunctionMaxLength {
    fn id(&self) -> &'static str { "detekt:naming:FunctionMaxLength" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        check_fn_name_len(tree, source, self.max, false)
    }
}

// ── FunctionMinLength ──
pub struct FunctionMinLength { min: usize }
impl FunctionMinLength { pub fn new() -> Self { Self { min: 3 } } }
impl Rule for FunctionMinLength {
    fn id(&self) -> &'static str { "detekt:naming:FunctionMinLength" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        check_fn_name_len(tree, source, self.min, true)
    }
}

fn check_fn_name_len(tree: &Tree, source: &str, threshold: usize, is_min: bool) -> Vec<Violation> {
    let mut v = Vec::new();
    let bytes = source.as_bytes();
    walk_fn_len(tree.root_node(), bytes, &mut v, threshold, is_min);
    v
}

fn walk_fn_len(n: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>, t: usize, is_min: bool) {
    if n.kind() == "function_declaration" {
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i) {
                if c.kind() == "simple_identifier" {
                    if let Ok(name) = c.utf8_text(bytes) {
                        let len = name.chars().count();
                        let exceeds = if is_min { len < t } else { len > t };
                        if exceeds {
                            let pos = c.start_position();
                            let rule = if is_min { "FunctionMinLength" } else { "FunctionMaxLength" };
                            let msg = if is_min {
                                format!("Function name \"{}\" is too short ({}, min {})", name, len, t)
                            } else {
                                format!("Function name \"{}\" is too long ({}, max {})", name, len, t)
                            };
                            v.push(Violation {
                                file: String::new(), line: pos.row + 1, col: pos.column + 1,
                                rule_id: format!("detekt:naming:{}", rule),
                                message: msg, auto_fixable: false,
                            });
                        }
                    }
                    break;
                }
            }
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { walk_fn_len(c, bytes, v, t, is_min); }
    }
}

// ── EnumNaming ──
pub struct EnumNaming;
impl Rule for EnumNaming {
    fn id(&self) -> &'static str { "detekt:naming:EnumNaming" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_enum_entries(tree.root_node(), source.as_bytes(), &mut v);
        v
    }
}

fn walk_enum_entries(n: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
    if n.kind() == "enum_entry" {
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i) {
                if c.kind() == "simple_identifier" {
                    if let Ok(name) = c.utf8_text(bytes) {
                        if !name.chars().all(|ch| ch.is_uppercase() || ch.is_ascii_digit() || ch == '_') {
                            let pos = c.start_position();
                            v.push(Violation {
                                file: String::new(), line: pos.row + 1, col: pos.column + 1,
                                rule_id: "detekt:naming:EnumNaming".into(),
                                message: format!("Enum entry \"{}\" should be UPPER_SNAKE_CASE", name),
                                auto_fixable: false,
                            });
                        }
                    }
                    break;
                }
            }
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { walk_enum_entries(c, bytes, v); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(r: &dyn Rule, s: &str) -> Vec<Violation> { r.check(&KotlinParser::new().parse(s), s) }

    #[test] fn fn_max_ok() { assert!(c(&FunctionMaxLength::new(), "fun abc() {}\n").is_empty()); }
    #[test] fn fn_max_bad() {
        assert!(!c(&FunctionMaxLength{max:5}, "fun abcdef() {}\n").is_empty());
    }
    #[test] fn fn_min_ok() { assert!(c(&FunctionMinLength::new(), "fun abc() {}\n").is_empty()); }
    #[test] fn fn_min_bad() {
        assert!(!c(&FunctionMinLength{min:5}, "fun ab() {}\n").is_empty());
    }
    #[test] fn enum_ok() { assert!(c(&EnumNaming, "enum class E { FOO, BAR }\n").is_empty()); }
    #[test] fn enum_bad() { assert!(!c(&EnumNaming, "enum class E { foo }\n").is_empty()); }
}
