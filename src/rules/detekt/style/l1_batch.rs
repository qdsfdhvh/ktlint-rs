//! detekt:style: L1 rules — LibraryCodeMustSpecifyReturnType, LibraryEntitiesShouldNotBePublic
use crate::resolver::builder::build_symbol_table;
use crate::rules::{Rule, Violation};

// ── LibraryCodeMustSpecifyReturnType ──
pub struct LibraryCodeMustSpecifyReturnType;

impl Rule for LibraryCodeMustSpecifyReturnType {
    fn id(&self) -> &'static str {
        "detekt:style:LibraryCodeMustSpecifyReturnType"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let table = build_symbol_table(source, tree.root_node());

        for sym in table.symbols.iter().filter(|s| {
            s.kind == crate::resolver::SymbolKind::Function
                && s.visibility != crate::resolver::Visibility::Private
        }) {
            // Check if the function declaration has an explicit return type
            let line = source.lines().nth(sym.line.saturating_sub(1)).unwrap_or("");
            let t = line.trim();
            // Public function without explicit return type (has `)` immediately followed by `{` or `=`)
            if t.contains("fun ")
                && !t.contains("): ")
                && !t.contains("):")
                && !t.ends_with(") {")
                && !t.ends_with("){")
            {
                // Functions with implicit return type that should specify it
                if t.contains(") {") || t.contains("){") {
                    // Has body but no return type specified
                } else {
                    continue;
                }
                violations.push(Violation {
                    file: String::new(),
                    line: sym.line,
                    col: sym.col,
                    rule_id: "detekt:style:LibraryCodeMustSpecifyReturnType".into(),
                    message: format!(
                        "Public function '{}' must specify return type explicitly",
                        sym.name
                    ),
                    auto_fixable: false,
                });
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn c(rule: &dyn Rule, s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        rule.check(&p.parse(s), s)
    }

    #[test]
    fn no_return_type_bad() {
        assert!(!c(
            &LibraryCodeMustSpecifyReturnType,
            "class Foo { fun bar() {}\n}\n"
        )
        .is_empty());
    }
    #[test]
    fn has_return_type_ok() {
        assert!(c(
            &LibraryCodeMustSpecifyReturnType,
            "class Foo { fun bar(): Int { return 1 }\n}\n"
        )
        .is_empty());
    }
    #[test]
    fn private_skip() {
        assert!(c(
            &LibraryCodeMustSpecifyReturnType,
            "class Foo { private fun bar() {}\n}\n"
        )
        .is_empty());
    }
}
