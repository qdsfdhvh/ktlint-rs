//! detekt:style:UnusedPrivateFunction — flags unused private functions.
use crate::resolver::Visibility;
use crate::rules::{Rule, Violation};
use std::collections::HashSet;

pub struct UnusedPrivateFunction;

impl Rule for UnusedPrivateFunction {
    fn id(&self) -> &'static str {
        "detekt:style:UnusedPrivateFunction"
    }

    fn check_with_symbols(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        sym: Option<&crate::resolver::SymbolTable>,
    ) -> Vec<Violation> {
        let table = sym.expect("SymbolTable should be provided by engine");
        let used: HashSet<String> = collect_refs(tree.root_node(), source);
        let mut v = Vec::new();
        for sym in table.symbols.iter().filter(|s| {
            s.visibility == Visibility::Private && s.kind == crate::resolver::SymbolKind::Function
        }) {
            if !used.contains(&sym.name) {
                v.push(Violation {
                    file: String::new(),
                    line: sym.line,
                    col: sym.col,
                    rule_id: "detekt:style:UnusedPrivateFunction".into(),
                    message: format!("Private function '{}' is never used", sym.name),
                    auto_fixable: false,
                });
            }
        }
        v
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        use crate::resolver::builder::build_symbol_table;
        let sym = build_symbol_table(source, tree.root_node());
        self.check_with_symbols(tree, source, Some(&sym))
    }
}

fn collect_refs(root: tree_sitter::Node, source: &str) -> HashSet<String> {
    let mut u = HashSet::new();
    let bytes = source.as_bytes();
    const DECL: &[&str] = &[
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
        let child_depth = if DECL.contains(&n.kind()) {
            Some(0)
        } else {
            decl_depth.map(|d| d + 1)
        };
        if !is_decl && (n.kind() == "simple_identifier" || n.kind() == "identifier") {
            if let Ok(name) = n.utf8_text(bytes) {
                if !name.starts_with('_') {
                    u.insert(name.to_string());
                }
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
    fn c(s: &str) -> Vec<Violation> {
        UnusedPrivateFunction.check(&KotlinParser::new().parse(s), s)
    }
    #[test]
    fn unused_func_bad() {
        assert!(!c("class Foo { private fun bar() {} }\n").is_empty());
    }
    #[test]
    fn used_func_ok() {
        assert!(c("class Foo { private fun bar() {}\n  fun f() { bar() } }\n").is_empty());
    }
}
