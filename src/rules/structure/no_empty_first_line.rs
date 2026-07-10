//! standard:no-empty-first-line-in-class-body — no blank first line after {
use crate::rules::{Rule, Violation};

pub struct NoEmptyFirstLineInClassBody;

impl Rule for NoEmptyFirstLineInClassBody {
    fn id(&self) -> &'static str {
        "standard:no-empty-first-line-in-class-body"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            // After `class Foo {` or `{`, next line shouldn't be empty
            if (trimmed.ends_with('{') && (trimmed.contains("class ") || trimmed.contains("fun ")))
                || trimmed == "{"
            {
                if i + 1 < lines.len() && lines[i + 1].trim().is_empty() {
                    violations.push(Violation {
                        file: String::new(),
                        line: i + 2,
                        col: 1,
                        rule_id: self.id().to_string(),
                        message: "Unexpected blank line at start of class body".to_string(),
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
    use super::*;
    use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        let t = p.parse(s);
        NoEmptyFirstLineInClassBody.check(&t, s)
    }
    #[test]
    fn no_blank_first_line() {
        assert!(check("class Foo {\n    val x = 1\n}\n").is_empty());
    }
    #[test]
    fn blank_first_line() {
        let v = check("class Foo {\n\n    val x = 1\n}\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:no-empty-first-line-in-class-body");
    }
}
