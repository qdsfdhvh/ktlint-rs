use crate::resolver::builder::build_symbol_table;
use crate::resolver::Visibility;
use crate::rules::{Rule, Violation};
use std::collections::HashSet;

pub struct UnusedPrivateMember;

impl Rule for UnusedPrivateMember {
    fn id(&self) -> &'static str { "detekt:naming:UnusedPrivateMember" }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let table = build_symbol_table(source, tree.root_node());
        let used: HashSet<String> = collect_references(tree.root_node(), source);

        for sym in table.symbols.iter().filter(|s| {
            s.visibility == Visibility::Private
                && matches!(
                    s.kind,
                    crate::resolver::SymbolKind::Function | crate::resolver::SymbolKind::Property
                )
        }) {
            if !used.contains(&sym.name) {
                violations.push(Violation {
                    file: String::new(), line: sym.line, col: sym.col,
                    rule_id: "detekt:naming:UnusedPrivateMember".into(),
                    message: format!("Private member '{}' is never used", sym.name),
                    auto_fixable: false,
                });
            }
        }
        violations
    }
}

fn collect_references(root: tree_sitter::Node, source: &str) -> HashSet<String> {
    let mut used = HashSet::new();
    let bytes = source.as_bytes();
    let mut stack = vec![root];

    const DECL_KINDS: &[&str] = &[
        "class_declaration", "function_declaration", "property_declaration",
        "object_declaration", "enum_entry", "type_alias",
        "import_header", "package_header",
        "class_parameter", "value_parameter",
    ];

    while let Some(node) = stack.pop() {
        let k = node.kind();
        if k == "simple_identifier" || k == "type_identifier" || k == "identifier" {
            if let Ok(name) = node.utf8_text(bytes) {
                let is_decl = node.parent().map_or(false, |p| DECL_KINDS.contains(&p.kind()));
                if !is_decl && !name.starts_with('_') {
                    used.insert(name.to_string());
                }
            }
        }
        for i in (0..node.child_count()).rev() {
            if let Some(c) = node.child(i) { stack.push(c); }
        }
    }

    used
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        UnusedPrivateMember.check(&p.parse(s), s)
    }

    #[test] fn unused_func_bad() { assert!(!c("class Foo {\n    private fun bar() {}\n}\n").is_empty()); }
    #[test] fn public_never_flagged() { assert!(c("class Foo {\n    fun bar() {}\n}\n").is_empty()); }
    #[test] fn private_val_used_ok() { assert!(c("class Foo {\n    private val x = 1\n    fun getX() = x\n}\n").is_empty()); }
    #[test] fn private_fun_used_ok() { assert!(c("class Foo {\n    private fun bar() {}\n    fun baz() { bar() }\n}\n").is_empty()); }
}
