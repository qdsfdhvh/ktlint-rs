//! detekt:style:UnusedPrivateClass — flags unused private classes.
use crate::resolver::builder::build_symbol_table;
use crate::rules::{Rule, Violation};
use std::collections::HashSet;

pub struct UnusedPrivateClass;

impl Rule for UnusedPrivateClass {
    fn id(&self) -> &'static str {
        "detekt:style:UnusedPrivateClass"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let table = build_symbol_table(source, tree.root_node());
        let used = refs(tree.root_node(), source);

        for sym in &table.symbols {
            if !matches!(
                sym.kind,
                crate::resolver::SymbolKind::Class | crate::resolver::SymbolKind::Object
            ) {
                continue;
            }
            if sym.visibility != crate::resolver::Visibility::Private {
                continue;
            }
            if used.contains(&sym.name) {
                continue;
            }
            v.push(Violation {
                file: String::new(),
                line: sym.line,
                col: sym.col,
                rule_id: "detekt:style:UnusedPrivateClass".into(),
                message: format!("Private class '{}' is never used", sym.name),
                auto_fixable: false,
            });
        }
        v
    }
}

fn refs(root: tree_sitter::Node, source: &str) -> HashSet<String> {
    let mut u = HashSet::new();
    let b = source.as_bytes();
    let mut s = vec![root];
    const D: &[&str] = &[
        "class_declaration",
        "function_declaration",
        "property_declaration",
        "parameter",
        "variable_declaration",
    ];
    while let Some(n) = s.pop() {
        if n.kind() == "type_identifier"
            || n.kind() == "simple_identifier"
            || n.kind() == "identifier"
        {
            if let Ok(name) = n.utf8_text(b) {
                if !n.parent().is_some_and(|p| D.contains(&p.kind())) {
                    u.insert(name.to_string());
                }
            }
        }
        for i in (0..n.child_count()).rev() {
            if let Some(c) = n.child(i) {
                s.push(c);
            }
        }
    }
    u
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        UnusedPrivateClass.check(&KotlinParser::new().parse(s), s)
    }
    #[test]
    fn unused_private_class() {
        assert!(!c("class Foo { private class Bar {}\n}\n").is_empty());
    }
    #[test]
    fn used_private_class() {
        assert!(c("class Foo { private class Bar {}\n    val x: Bar = Bar()\n}\n").is_empty());
    }
}
