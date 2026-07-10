//! standard:spacing-around-range-operator — spaces around .. operator.
use crate::rules::{Rule, Violation};

pub struct RangeOperatorSpacing;

impl Rule for RangeOperatorSpacing {
    fn id(&self) -> &'static str {
        "standard:spacing-around-range-operator"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();

        let mut i = 0;
        while i < bytes.len() {
            if i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1] == b'.' {
                // Found `..`
                let line_num = source[..i].matches('\n').count() + 1;
                let col = if let Some(last_nl) = source[..i].rfind('\n') {
                    i - last_nl
                } else {
                    i + 1
                };

                // Check space before
                if i > 0 && bytes[i - 1] == b' ' && i > 1 && bytes[i - 2] != b'.' {
                    violations.push(Violation {
                        file: String::new(),
                        line: line_num,
                        col,
                        rule_id: self.id().to_string(),
                        message: "Unexpected space before \"..\"".to_string(),
                        auto_fixable: true,
                    });
                }
                // Check space after
                if i + 2 < bytes.len() && bytes[i + 2] == b' ' {
                    violations.push(Violation {
                        file: String::new(),
                        line: line_num,
                        col: col + 2,
                        rule_id: self.id().to_string(),
                        message: "Unexpected space after \"..\"".to_string(),
                        auto_fixable: true,
                    });
                }
            }
            i += 1;
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
        RangeOperatorSpacing.check(&t, s)
    }
    #[test]
    fn range_ok() {
        assert!(check("for (i in 1..10)\n").is_empty());
    }
    #[test]
    fn space_before_range() {
        let v = check("for (i in 1 ..10)\n");
        assert!(!v.is_empty());
        assert!(v.iter().any(|x| x.message.contains("before")));
    }
}
