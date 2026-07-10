//! standard:indent — enforce 4-space indentation (configurable via .editorconfig).

use crate::rules::{Rule, Violation};

pub struct Indentation;

impl Rule for Indentation {
    fn id(&self) -> &'static str {
        "standard:indent"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let indent_size: usize = 4; // configurable

        for (i, line) in source.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            // Tab indentation
            if line.starts_with('\t') {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().to_string(),
                    message: "Unexpected tab character(s)".to_string(),
                    auto_fixable: true,
                });
                continue;
            }

            // Check indentation is a multiple of indent_size
            let spaces = line.chars().take_while(|c| *c == ' ').count();
            if spaces > 0 && spaces % indent_size != 0 {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().to_string(),
                    message: format!(
                        "Unexpected indentation ({}) — should be a multiple of {}",
                        spaces, indent_size
                    ),
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
        let tree = parser.parse(source);
        Indentation.check(&tree, source)
    }

    #[test]
    fn correct_indentation() {
        assert!(check("fun foo() {\n    val x = 1\n}\n").is_empty());
    }

    #[test]
    fn incorrect_indentation() {
        let v = check("fun foo() {\n   val x = 1\n}\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:indent");
    }

    #[test]
    fn tab_indentation() {
        let v = check("fun foo() {\n\tval x = 1\n}\n");
        assert!(!v.is_empty());
    }
}
