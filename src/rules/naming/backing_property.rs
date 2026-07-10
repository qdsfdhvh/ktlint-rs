//! standard:backing-property-naming — backing properties prefixed with underscore.
use crate::rules::{Rule, Violation};

pub struct BackingPropertyNaming;

impl Rule for BackingPropertyNaming {
    fn id(&self) -> &'static str { "standard:backing-property-naming" }
    fn auto_fixable(&self) -> bool { false }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("private val _") || t.starts_with("private var _") {
                // Backing property pattern: `private var _items = mutableListOf<Item>()`
                // followed by `val items: List<Item> get() = _items`
                // This is fine — underscore prefix is convention
            } else if t.starts_with("val _") && !t.contains("_items") {
                // Public backing property — suspicious
                violations.push(Violation {
                    file: String::new(), line: i + 1, col: 1,
                    rule_id: self.id().to_string(),
                    message: "Public property should not start with underscore".to_string(),
                    auto_fixable: false,
                });
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*; use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> { let mut p=KotlinParser::new(); BackingPropertyNaming.check(&p.parse(s), s) }
    #[test] fn normal_val() { assert!(check("val items = listOf()\n").is_empty()); }
    #[test] fn public_underscore() { let v=check("val _count = 0\n"); assert!(!v.is_empty()); }
}
