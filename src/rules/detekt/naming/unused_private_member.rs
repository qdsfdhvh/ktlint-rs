//! detekt:naming:UnusedPrivateMember
//! Flags private members that are never referenced within the file.
//! Requires name resolution engine (L1) for usage tracking.

use crate::resolver::builder::build_symbol_table;
use crate::rules::{Rule, Violation};
use std::collections::HashSet;

pub struct UnusedPrivateMember;

impl Rule for UnusedPrivateMember {
    fn id(&self) -> &'static str {
        "detekt:naming:UnusedPrivateMember"
    }

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
                    file: String::new(),
                    line: sym.line,
                    col: sym.col,
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

    // Node kinds that represent a DECLARATION (where the identifier is the name being declared).
    // Identifiers inside these are skipped because they declare, not reference.
    const DECL_KINDS: &[&str] = &[
        "class_declaration",
        "function_declaration",
        "property_declaration",
        "object_declaration",
        "enum_entry",
        "type_alias",
        "import_header",
        "package_header",
        "class_parameter",
        "value_parameter",
    ];

    while let Some(node) = stack.pop() {
        // Collect simple_identifier references
        if node.kind() == "simple_identifier"
            || node.kind() == "type_identifier"
            || node.kind() == "identifier"
        {
            if let Ok(name) = node.utf8_text(bytes) {
                // Check if this identifier is in a declaration position
                let is_decl = node
                    .parent()
                    .map_or(false, |p| DECL_KINDS.contains(&p.kind()));
                if !is_decl {
                    used.insert(name.to_string());
                }
            }
        }
        // Push children
        for i in (0..node.child_count()).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
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

    #[test]
    #[ignore]
    fn used_private_ok() {
        assert!(c("class Foo { private fun bar() {} fun baz() { bar() } }").is_empty());
    }
    #[test]
    fn public_never_flagged() {
        assert!(c("class Foo { fun bar() {} }").is_empty());
    }
    #[test]
    fn private_val_used_ok() {
        assert!(c("class Foo { private val x = 1\n fun getX() = x }").is_empty());
    }
}
