use crate::resolver::builder::build_symbol_table;
use crate::rules::{Rule, Violation};

pub struct LibraryCodeMustSpecifyReturnType;

impl Rule for LibraryCodeMustSpecifyReturnType {
    fn id(&self) -> &'static str {
        "detekt:style:LibraryCodeMustSpecifyReturnType"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let table = build_symbol_table(source, tree.root_node());
        for sym in &table.symbols {
            if sym.kind != crate::resolver::SymbolKind::Function {
                continue;
            }
            if sym.visibility == crate::resolver::Visibility::Private {
                continue;
            }
            let line = source.lines().nth(sym.line.saturating_sub(1)).unwrap_or("");
            if line.contains("fun ") && !line.contains("): ") && !line.contains("):") {
                violations.push(Violation {
                    file: String::new(),
                    line: sym.line,
                    col: sym.col,
                    rule_id: "detekt:style:LibraryCodeMustSpecifyReturnType".into(),
                    message: format!("'{}' must specify return type explicitly", sym.name),
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
    fn c(s: &str) -> Vec<Violation> {
        LibraryCodeMustSpecifyReturnType.check(&KotlinParser::new().parse(s), s)
    }
    #[test]
    fn no_return_type() {
        assert!(!c("class Foo { fun bar() {}\n}\n").is_empty());
    }
    #[test]
    fn has_return_type() {
        assert!(c("class Foo { fun bar(): Int { return 1 }\n}\n").is_empty());
    }
}
