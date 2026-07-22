//! standard:spacing-around-keyword — spaces around if/for/while/try/catch keywords.
use crate::rules::{Rule, Violation};

pub struct SpacingAroundKeyword;

impl Rule for SpacingAroundKeyword {
    fn id(&self) -> &'static str {
        "standard:spacing-around-keyword"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let keywords = ["if", "for", "while", "try", "catch", "when"];

        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            for kw in &keywords {
                let pattern = format!("{}(", kw);
                if let Some(pos) = trimmed.find(&pattern) {
                    // Check: keyword should be preceded by space or line start
                    // Must be a whole keyword, not substring of identifier
                    let is_keyword =
                        pos == 0 || !trimmed.as_bytes()[pos - 1].is_ascii_alphanumeric();
                    if is_keyword && pos > 0 && trimmed.as_bytes()[pos - 1] != b' ' {
                        violations.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: pos + 1,
                            rule_id: self.id().to_string(),
                            message: format!("Missing space before \"{}\"", kw),
                            auto_fixable: true,
                        });
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
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        let t = p.parse(s);
        SpacingAroundKeyword.check(&t, s)
    }
    #[test]
    fn valid_keyword() {
        assert!(check("if (x) {}\n").is_empty());
    }
    #[test]
    fn missing_space() {
        let v = check("val x=if(true)1 else 2\n");
        assert!(!v.is_empty());
    }
}
