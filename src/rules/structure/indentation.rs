//! standard:indent — enforce consistent indentation (configurable via .editorconfig).

use crate::rules::{Rule, Violation};

pub struct Indentation {
    indent_size: usize,
}

impl Indentation {
    pub fn new(indent_size: usize) -> Self {
        Self { indent_size }
    }
}

impl Rule for Indentation {
    fn id(&self) -> &'static str {
        "standard:indent"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let indent_size = self.indent_size;

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
        Indentation::new(4).check(&tree, source)
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

    #[test]
    fn two_space_indent() {
        let rule = Indentation::new(2);
        let mut parser = KotlinParser::new();
        let source = "fun foo() {\n  val x = 1\n}\n";
        let tree = parser.parse(source);
        assert!(rule.check(&tree, source).is_empty());
    }
}
