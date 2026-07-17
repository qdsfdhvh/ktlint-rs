//! detekt:naming:ConstructorParameterNaming — already L0, enhanced with resolver.
//! detekt:style:ExplicitApiVisibility — requires all public API to have explicit visibility.
//! Uses name resolution engine (L1).

use crate::resolver::Visibility;
use crate::rules::{Rule, Violation};

pub struct ExplicitApiVisibility;

impl Rule for ExplicitApiVisibility {
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        self.check_with_symbols(tree, source, None)
    }

    fn id(&self) -> &'static str {
        "detekt:style:ExplicitApiVisibility"
    }

    fn check_with_symbols(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        sym: Option<&crate::resolver::SymbolTable>,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();
        let table = sym.expect("SymbolTable should be provided by engine");

        // Flag declarations with implicit visibility at top-level or class level
        for sym in &table.symbols {
            if sym.visibility != Visibility::Implicit {
                continue;
            }
            // Only check class/function/property — not local vars
            match sym.kind {
                crate::resolver::SymbolKind::Class
                | crate::resolver::SymbolKind::Function
                | crate::resolver::SymbolKind::Property => {}
                _ => continue,
            }
            // Skip if it's inside a function body (local declarations don't need visibility)
            let scope = &table.scopes[sym.scope_id];
            if scope.parent_id.map_or(false, |pid| {
                table.scopes[pid].parent_id.is_some() // nested deeper than class level
            }) {
                continue;
            }
            violations.push(Violation {
                file: String::new(),
                line: sym.line,
                col: sym.col,
                rule_id: "detekt:style:ExplicitApiVisibility".into(),
                message: format!(
                    "Declaration '{}' should have explicit visibility modifier",
                    sym.name
                ),
                auto_fixable: false,
            });
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        let tree = p.parse(s);
        ExplicitApiVisibility.check(&tree, s)
    }

    #[test]
    fn implicitly_public_bad() {
        assert!(!c("class Foo {}\n").is_empty());
    }
    #[test]
    fn explicitly_public_ok() {
        assert!(c("public class Foo {}\n").is_empty());
    }
    #[test]
    fn private_class_ok() {
        assert!(c("private class Foo { private fun bar() {} }\n").is_empty());
    }
    #[test]
    fn local_val_ok() {
        assert!(c("private fun foo() { val x = 1 }\n").is_empty());
    }
}
