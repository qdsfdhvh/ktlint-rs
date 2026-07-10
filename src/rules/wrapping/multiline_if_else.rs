//! standard:multiline-if-else — closing brace and else/newline consistency.
//!
//! Checks:
//! - `} else {` must be on same line
//! - `} else if {` must be on same line
//! - `else` on its own line after `}` is a violation

use crate::rules::{Rule, Violation};

pub struct MultilineIfElse;

impl Rule for MultilineIfElse {
    fn id(&self) -> &'static str {
        "standard:multiline-if-else"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Lines starting with `else` must be on the same line as preceding `}`
            if trimmed.starts_with("else") {
                if i > 0 {
                    let prev_trimmed = lines[i - 1].trim();
                    // OK: `} else {`, `} else if (...)`, `} else`
                    if !prev_trimmed.ends_with('}') || !prev_trimmed.contains("else") {
                        // Previous line ends with `}` and doesn't contain `else`
                        // → this `else` is on a separate line by itself
                        violations.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: self.id().to_string(),
                            message:
                                "\"else\" should be on same line as preceding \"}\"".to_string(),
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

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        MultilineIfElse.check(&tree, source)
    }

    #[test]
    fn else_on_same_line() {
        assert!(check("if (x) {\n    doA()\n} else {\n    doB()\n}\n").is_empty());
    }

    #[test]
    fn else_on_new_line_bad() {
        let v = check("if (x) {\n    doA()\n}\nelse {\n    doB()\n}\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:multiline-if-else");
    }

    #[test]
    fn else_if_on_same_line() {
        assert!(
            check("if (x) {\n    doA()\n} else if (y) {\n    doB()\n}\n").is_empty()
        );
    }
}
