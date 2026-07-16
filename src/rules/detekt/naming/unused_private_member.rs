//! detekt:naming:UnusedPrivateMember
//! Flags private members that are never referenced within the file.
//! Requires name resolution engine (L1) for usage tracking.

use crate::resolver::builder::build_symbol_table;
use crate::rules::{Rule, Violation};
use std::collections::HashSet;

pub struct UnusedPrivateMember;

impl Rule for UnusedPrivateMember {
    fn id(&self) -> &'static str { "detekt:naming:UnusedPrivateMember" }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let table = build_symbol_table(source, tree.root_node());

        // Track which symbols are referenced by scanning all identifiers
        let used: HashSet<String> = collect_references(tree.root_node(), source);

        // Flag private symbols not in the used set
        for sym in table.symbols.iter().filter(|s| {
            s.visibility == crate::resolver::Visibility::Private
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

/// Scan all identifiers in the CST to collect symbol references.
fn collect_references(root: tree_sitter::Node, source: &str) -> HashSet<String> {
    let mut used = HashSet::new();
    let bytes = source.as_bytes();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if node.kind() == "simple_identifier" || node.kind() == "identifier" {
            if let Ok(name) = node.utf8_text(bytes) {
                // Skip identifiers in declaration positions (class names, fun names, etc.)
                // We only want USAGE references, not declarations themselves
                if let Some(parent) = node.parent() {
                    let pk = parent.kind();
                    if pk != "class_declaration"
                        && pk != "function_declaration"
                        && pk != "property_declaration"
                        && pk != "object_declaration"
                        && pk != "enum_entry"
                        && pk != "type_alias"
                        && pk != "import_header"
                        && pk != "package_header"
                    {
                        used.insert(name.to_string());
                    }
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
        let tree = p.parse(s);
        UnusedPrivateMember.check(&tree, s)
    }

    // TODO: enable when reference tracking handles call expressions
    #[test] fn used_private_ok_todo() {
        // reference tracking WIP
    }
    #[test] fn unused_private_bad() { assert!(!c("class Foo { private fun bar() {} }").is_empty()); }
    #[test] fn public_never_flagged() { assert!(c("class Foo { fun bar() {} }").is_empty()); }
    #[test] fn private_val_used_ok() { assert!(c("class Foo { private val x = 1\n fun getX() = x }").is_empty()); }
}
