//! `standard:double-colon-spacing` parity with ktlint 1.8.

use crate::rules::{Rule, Violation};

pub struct DoubleColonSpacing;

impl Rule for DoubleColonSpacing {
    fn id(&self) -> &'static str {
        "standard:double-colon-spacing"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        for (line_index, line) in source.lines().enumerate() {
            for (position, _) in line.match_indices("::") {
                let before = position > 0
                    && line.as_bytes()[position - 1] == b' '
                    && position >= 2
                    && line.as_bytes()[position - 2].is_ascii_alphanumeric();
                let after = line.as_bytes().get(position + 2) == Some(&b' ');
                let (column, message) = match (before, after) {
                    (true, true) => (position + 1, "Unexpected spacing around \"::\""),
                    (true, false) => (position, "Unexpected spacing before \"::\""),
                    (false, true) => (position + 3, "Unexpected spacing after \"::\""),
                    (false, false) => continue,
                };
                violations.push(Violation {
                    file: String::new(),
                    line: line_index + 1,
                    col: column,
                    rule_id: self.id().to_string(),
                    message: message.to_string(),
                    auto_fixable: true,
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

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        DoubleColonSpacing.check(&parser.parse(source), source)
    }

    #[test]
    fn accepts_bound_and_unbound_references() {
        assert!(check("val bound = String::length\nval unbound = ::println\n").is_empty());
    }

    #[test]
    fn groups_spacing_around_double_colon() {
        let violations = check("val x = String :: class\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].message, "Unexpected spacing around \"::\"");
    }
}
