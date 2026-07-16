//! detekt:style:RedundantModifier
//! Flags redundant visibility/modality modifiers.
//! Uses name resolution engine (L1).

use crate::resolver::builder::build_symbol_table;
use crate::resolver::Visibility;
use crate::rules::{Rule, Violation};

pub struct RedundantModifier;

impl Rule for RedundantModifier {
    fn id(&self) -> &'static str {
        "detekt:style:RedundantModifier"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let table = build_symbol_table(source, tree.root_node());

        // Check for public members inside private/sealed/internal classes
        for sym in &table.symbols {
            if sym.visibility != Visibility::Public {
                continue;
            }
            // Find enclosing class
            if let Some(scope) = table.scopes.get(sym.scope_id) {
                if let Some(parent_id) = scope.parent_id {
                    for class_sym in table.symbols.iter().filter(|s| {
                        matches!(
                            s.kind,
                            crate::resolver::SymbolKind::Class
                                | crate::resolver::SymbolKind::Object
                        ) && s.scope_id == parent_id
                    }) {
                        // Public member in private/internal/sealed class is effectively hidden
                        if class_sym.visibility == Visibility::Private
                            || class_sym.visibility == Visibility::Internal
                        {
                            violations.push(Violation {
                                file: String::new(),
                                line: sym.line,
                                col: sym.col,
                                rule_id: "detekt:style:RedundantModifier".into(),
                                message: format!(
                                    "Member '{}' is public but enclosed in a '{}' class '{}'",
                                    sym.name,
                                    format!("{:?}", class_sym.visibility).to_lowercase(),
                                    class_sym.name
                                ),
                                auto_fixable: false,
                            });
                        }
                    }
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
        let mut p = KotlinParser::new();
        let tree = p.parse(s);
        RedundantModifier.check(&tree, s)
    }

    #[test]
    fn public_class_ok() {
        assert!(c("public class Foo { public fun bar() {} }").is_empty());
    }
    #[test]
    fn public_in_private_bad() {
        assert!(!c("private class Foo { public fun bar() {} }").is_empty());
    }
    #[test]
    fn private_in_private_ok() {
        assert!(c("private class Foo { private fun bar() {} }").is_empty());
    }
}
