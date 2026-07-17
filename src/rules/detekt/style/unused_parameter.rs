//! detekt:style:UnusedParameter — flags unused function parameters.
//! Perf: no Node::parent() calls; uses DFS flag propagation.

use crate::resolver::builder::build_symbol_table;
use crate::rules::{Rule, Violation};
use std::collections::HashSet;

pub struct UnusedParameter;

impl Rule for UnusedParameter {
    fn id(&self) -> &'static str {
        "detekt:style:UnusedParameter"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let table = build_symbol_table(source, tree.root_node());
        let used = refs(tree.root_node(), source);
        for sym in &table.symbols {
            if sym.kind != crate::resolver::SymbolKind::Property {
                continue;
            }
            if table.scopes[sym.scope_id].parent_id.is_none() {
                continue;
            }
            if table.enclosing_class_scope(sym.scope_id).is_some() {
                continue;
            }
            if used.contains(&sym.name) {
                continue;
            }
            v.push(Violation {
                file: String::new(),
                line: sym.line,
                col: sym.col,
                rule_id: "detekt:style:UnusedParameter".into(),
                message: format!("'{}' is never used", sym.name),
                auto_fixable: false,
            });
        }
        v
    }
}

/// Collect all simple_identifier references, skipping declaration names.
/// Uses DFS flag propagation instead of Node::parent() calls.
fn refs(root: tree_sitter::Node, source: &str) -> HashSet<String> {
    let mut u = HashSet::new();
    let b = source.as_bytes();
    const D: &[&str] = &[
        "class_declaration",
        "function_declaration",
        "property_declaration",
        "parameter",
        "variable_declaration",
        "class_parameter",
        "value_parameter",
    ];
    let mut stack: Vec<(_, Option<usize>)> = vec![(root, None)];
    while let Some((n, decl_depth)) = stack.pop() {
        let is_decl = decl_depth == Some(0);
        let child_depth = if D.contains(&n.kind()) { Some(0) } else { decl_depth.map(|d| d + 1) };
        if !is_decl
            && (n.kind() == "simple_identifier" || n.kind() == "identifier")
        {
            if let Ok(name) = n.utf8_text(b) {
                u.insert(name.to_string());
            }
        }
        for i in (0..n.child_count()).rev() {
            if let Some(c) = n.child(i) {
                stack.push((c, child_depth));
            }
        }
    }
    u
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    #[test]
    fn builder_tracks_param() {
        let src = "fun f(x: Int) {}";
        let tree = KotlinParser::new().parse(src);
        let table = build_symbol_table(src, tree.root_node());
        let used = refs(tree.root_node(), src);
        let unused: Vec<_> = table
            .symbols
            .iter()
            .filter(|s| s.kind == crate::resolver::SymbolKind::Property && !used.contains(&s.name))
            .map(|s| s.name.clone())
            .collect();
        assert!(unused.contains(&"x".into()), "unused params: {:?}", unused);
    }
}
