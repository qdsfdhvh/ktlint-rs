//! detekt:naming:ProtectedMemberInFinalClass
//! Flags protected members in classes that cannot be subclassed (final, non-open).
//! Requires name resolution engine (L1) to track class modifiers and member visibility.

use crate::rules::{Rule, Violation};

pub struct ProtectedMemberInFinalClass;

impl Rule for ProtectedMemberInFinalClass {
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        {
            use crate::resolver::builder::build_symbol_table;
            let sym = build_symbol_table(source, tree.root_node());
            self.check_with_symbols(tree, source, Some(&sym))
        }
    }

    fn id(&self) -> &'static str {
        "detekt:naming:ProtectedMemberInFinalClass"
    }

    fn check_with_symbols(
        &self,
        _tree: &tree_sitter::Tree,
        _source: &str,
        sym: Option<&crate::resolver::SymbolTable>,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();
        let table = sym.expect("SymbolTable should be provided by engine");

        // Find all protected symbols
        for sym in table
            .symbols
            .iter()
            .filter(|s| s.visibility == crate::resolver::Visibility::Protected)
        {
            // Walk up to find the enclosing class
            if let Some(scope) = table.scopes.get(sym.scope_id) {
                if let Some(parent_id) = scope.parent_id {
                    for class_sym in table.symbols.iter().filter(|s| {
                        matches!(
                            s.kind,
                            crate::resolver::SymbolKind::Class
                                | crate::resolver::SymbolKind::Object
                        ) && s.scope_id == parent_id
                    }) {
                        // Check if the class is effectively final (not open, not abstract, not sealed)
                        // We determine this from the source text at class declaration
                        if !is_extensible_class(class_sym.line, source) {
                            violations.push(Violation {
                                file: String::new(),
                                line: sym.line,
                                col: sym.col,
                                rule_id: "detekt:naming:ProtectedMemberInFinalClass".into(),
                                message: format!(
                                    "Protected member '{}' in final class '{}'",
                                    sym.name, class_sym.name
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

fn is_extensible_class(line: usize, source: &str) -> bool {
    // Check if the class declaration line contains "open", "abstract", or "sealed"
    if let Some(class_line) = source.lines().nth(line.saturating_sub(1)) {
        let t = class_line.trim();
        t.starts_with("open ")
            || t.starts_with("abstract ")
            || t.starts_with("sealed ")
            || t.starts_with("interface ")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        let tree = p.parse(s);
        ProtectedMemberInFinalClass.check(&tree, s)
    }

    #[test]
    fn final_class_protected_bad() {
        assert!(!c("class Foo { protected fun bar() {} }").is_empty());
    }
    #[test]
    fn open_class_protected_ok() {
        assert!(c("open class Foo { protected fun bar() {} }").is_empty());
    }
    #[test]
    fn public_member_ok() {
        assert!(c("class Foo { fun bar() {} }").is_empty());
    }
}
