//! standard:spacing-around-double-colon — no spaces around ::.
use crate::rules::{Rule, Violation};

pub struct DoubleColonSpacing;

impl Rule for DoubleColonSpacing {
    fn id(&self) -> &'static str { "standard:spacing-around-double-colon" }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();

        for (i, line) in source.lines().enumerate() {
            if let Some(pos) = line.find("::") {
                let byte_pos = source.lines().take(i).map(|l| l.len() + 1).sum::<usize>() + pos;
                // Check space before ::
                if byte_pos > 0 && bytes[byte_pos - 1] == b' ' {
                    violations.push(Violation {
                        file: String::new(), line: i + 1, col: pos + 1,
                        rule_id: self.id().to_string(),
                        message: "Unexpected space before \"::\"".to_string(),
                        auto_fixable: true,
                    });
                }
                // Check space after ::
                if byte_pos + 2 < bytes.len() && bytes[byte_pos + 2] == b' ' {
                    violations.push(Violation {
                        file: String::new(), line: i + 1, col: pos + 3,
                        rule_id: self.id().to_string(),
                        message: "Unexpected space after \"::\"".to_string(),
                        auto_fixable: true,
                    });
                }
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*; use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new(); let t = p.parse(s); DoubleColonSpacing.check(&t, s)
    }
    #[test] fn double_colon_ok() { assert!(check("val x = String::class\n").is_empty()); }
    #[test] fn space_before() {
        let v = check("val x = String ::class\n");
        assert!(!v.is_empty());
        assert!(v.iter().any(|x| x.message.contains("before")));
    }
}
