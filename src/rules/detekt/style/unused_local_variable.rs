//! detekt:style:UnusedLocalVariable — flags unused local variables.
//! Requires function_body scope tracking (L1).
//! Perf: no Node::parent() calls; uses DFS flag propagation.

use crate::rules::{Rule, Violation};
use std::collections::HashSet;

pub struct UnusedLocalVariable;

impl Rule for UnusedLocalVariable {
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        {
            use crate::resolver::builder::build_symbol_table;
            let sym = build_symbol_table(source, tree.root_node());
            self.check_with_symbols(tree, source, Some(&sym))
        }
    }

    fn id(&self) -> &'static str {
        "detekt:style:UnusedLocalVariable"
    }

    fn check_with_symbols(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        sym: Option<&crate::resolver::SymbolTable>,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();
        let table = sym.expect("SymbolTable should be provided by engine");

        // Collect all identifier references — only skip the _direct_ child
        // of a declaration node (depth == 0), not grandchildren.
        let used: HashSet<String> = {
            let mut u = HashSet::new();
            let bytes = source.as_bytes();
            const DECL: &[&str] = &[
                "variable_declaration",
                "value_parameter",
                "class_declaration",
                "function_declaration",
                "property_declaration",
            ];
            let mut stack: Vec<(_, Option<usize>)> = vec![(tree.root_node(), None)];
            while let Some((node, decl_depth)) = stack.pop() {
                let is_decl = decl_depth == Some(0);
                let child_depth = if DECL.contains(&node.kind()) {
                    Some(0)
                } else {
                    decl_depth.map(|d| d + 1)
                };
                if !is_decl && (node.kind() == "simple_identifier" || node.kind() == "identifier") {
                    if let Ok(name) = node.utf8_text(bytes) {
                        if !name.starts_with('_') {
                            u.insert(name.to_string());
                        }
                    }
                }
                for i in (0..node.child_count()).rev() {
                    if let Some(c) = node.child(i) {
                        stack.push((c, child_depth));
                    }
                }
            }
            u
        };

        // Find variables inside function bodies
        for scope in &table.scopes {
            if scope.parent_id.is_none() || scope.parent_id == Some(0) {
                continue;
            }
            for &sym_id in &scope.symbols {
                let sym = &table.symbols[sym_id];
                if sym.kind == crate::resolver::SymbolKind::Property && !used.contains(&sym.name) {
                    violations.push(Violation {
                        file: String::new(),
                        line: sym.line,
                        col: sym.col,
                        rule_id: "detekt:style:UnusedLocalVariable".into(),
                        message: format!("Local variable '{}' is never used", sym.name),
                        auto_fixable: false,
                    });
                }
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn c(s: &str) -> Vec<Violation> {
        UnusedLocalVariable.check(&KotlinParser::new().parse(s), s)
    }

    #[test]
    fn unused_local_bad() {
        assert!(!c("fun foo() { val x = 1 }").is_empty());
    }
    #[test]
    fn used_local_ok() {
        assert!(c("fun foo() { val x = 1; println(x) }").is_empty());
    }
}
