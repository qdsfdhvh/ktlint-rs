//! detekt:style:ConstructorsOnTop — constructors should be first in class body.
use crate::rules::{Rule, Violation};

pub struct ConstructorsOnTop;

impl Rule for ConstructorsOnTop {
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        {
            use crate::resolver::builder::build_symbol_table;
            let sym = build_symbol_table(source, tree.root_node());
            self.check_with_symbols(tree, source, Some(&sym))
        }
    }

    fn id(&self) -> &'static str {
        "detekt:style:ConstructorsOnTop"
    }
    fn check_with_symbols(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        sym: Option<&crate::resolver::SymbolTable>,
    ) -> Vec<Violation> {
        let mut v = Vec::new();
        let table = sym.expect("SymbolTable should be provided by engine");

        // For each scope that contains a constructor, check if any properties/functions appear before it
        for scope in &table.scopes {
            let symbols: Vec<_> = scope
                .symbols
                .iter()
                .filter_map(|&id| table.symbols.get(id))
                .collect();
            if !symbols
                .iter()
                .any(|s| s.kind == crate::resolver::SymbolKind::Constructor)
            {
                continue;
            }
            let mut saw_non_ctor = false;
            for sym in &symbols {
                match sym.kind {
                    crate::resolver::SymbolKind::Constructor => {
                        if saw_non_ctor {
                            v.push(Violation {
                                file: String::new(),
                                line: sym.line,
                                col: sym.col,
                                rule_id: "detekt:style:ConstructorsOnTop".into(),
                                message: "Constructor should be declared before other members"
                                    .into(),
                                auto_fixable: false,
                            });
                        }
                    }
                    crate::resolver::SymbolKind::Property
                    | crate::resolver::SymbolKind::Function => {
                        saw_non_ctor = true;
                    }
                    _ => {}
                }
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        ConstructorsOnTop.check(&KotlinParser::new().parse(s), s)
    }
    #[test]
    fn ctor_on_top_ok() {
        assert!(c("class Foo {\n    constructor() {}\n    fun bar() {}\n}\n").is_empty());
    }
    #[test]
    fn ctor_not_first_bad() {
        assert!(!c("class Foo {\n    fun bar() {}\n    constructor() {}\n}\n").is_empty());
    }
}
