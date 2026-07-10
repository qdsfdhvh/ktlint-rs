//! standard:no-blank-line-before-rbrace — no blank lines before closing `}`.

use crate::rules::{Rule, Violation};

pub struct NoBlankLineBeforeRbrace;

impl Rule for NoBlankLineBeforeRbrace {
    fn id(&self) -> &'static str {
        "standard:no-blank-line-before-rbrace"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed == "}" {
                // Check if the previous non-empty line is also `}` or if there's a blank line
                if i > 0 {
                    let mut prev_idx = i - 1;
                    loop {
                        if lines[prev_idx].trim().is_empty() {
                            violations.push(Violation {
                                file: String::new(),
                                line: i + 1,
                                col: 1,
                                rule_id: self.id().to_string(),
                                message: "Blank line(s) before \"}\"".to_string(),
                                auto_fixable: true,
                            });
                            break;
                        }
                        if prev_idx == 0 {
                            break;
                        }
                        prev_idx -= 1;
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

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        NoBlankLineBeforeRbrace.check(&tree, source)
    }

    #[test]
    fn no_blank_line_before_brace() {
        assert!(check("fun foo() {\n    val x = 1\n}\n").is_empty());
    }

    #[test]
    fn blank_line_before_brace() {
        let v = check("fun foo() {\n    val x = 1\n\n}\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:no-blank-line-before-rbrace");
    }
}
