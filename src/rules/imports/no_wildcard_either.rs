//! standard:no-wildcard-imports-either — wildcard imports detection for Multiplatform.
use crate::rules::{Rule, Violation};

pub struct NoWildcardImportsEither;

impl Rule for NoWildcardImportsEither {
    fn id(&self) -> &'static str {
        "standard:no-wildcard-imports-either"
    }
    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter(|(_, l)| {
                l.trim().starts_with("import ") && l.contains(".*") && !l.trim().contains("//")
            })
            .map(|(i, _)| Violation {
                file: String::new(),
                line: i + 1,
                col: 1,
                rule_id: self.id().to_string(),
                message: "Wildcard import (either)".to_string(),
                auto_fixable: false,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        NoWildcardImportsEither.check(&p.parse(s), s)
    }
    #[test]
    fn wildcard() {
        let v = check("import foo.*\n");
        assert!(!v.is_empty());
    }
    #[test]
    fn no_wildcard() {
        assert!(check("import foo.Bar\n").is_empty());
    }
}
