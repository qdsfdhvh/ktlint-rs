//! standard:no-leading-empty-lines-in-method — no blank lines at method start.
use crate::rules::{Rule, Violation};

pub struct NoLeadingEmptyLinesInMethod;

impl Rule for NoLeadingEmptyLinesInMethod {
    fn id(&self) -> &'static str {
        "standard:no-leading-empty-lines-in-method"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if (t.starts_with("fun ") || t.starts_with("init {"))
                && t.ends_with('{')
                && i + 1 < lines.len()
                && lines[i + 1].trim().is_empty()
            {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().to_string(),
                    message: "Unexpected blank line at start of method body".to_string(),
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
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        NoLeadingEmptyLinesInMethod.check(&p.parse(s), s)
    }
    #[test]
    fn no_blank() {
        assert!(check("fun foo() {\n    doA()\n}\n").is_empty());
    }
    #[test]
    fn has_blank() {
        let v = check("fun foo() {\n\n    doA()\n}\n");
        assert!(!v.is_empty());
    }
}
