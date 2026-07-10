//! standard:no-single-expression-body — enforce braces for multi-line expression bodies.

use crate::rules::{Rule, Violation};

pub struct NoSingleExpressionBody;

impl Rule for NoSingleExpressionBody {
    fn id(&self) -> &'static str {
        "standard:no-single-expression-body"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Detect expression-body functions that span lines
            // Pattern: `fun name(params): Type = expr`
            if trimmed.starts_with("fun ") && trimmed.contains('=') {
                // Check if the `=` is followed by content on the same line
                if let Some(eq_pos) = trimmed.find('=') {
                    let after = trimmed[eq_pos + 1..].trim();
                    if !after.is_empty() && !after.starts_with('{') {
                        // Expression body on same line — OK
                    }
                }

                // Check if next line continues the expression
                if i + 1 < lines.len() {
                    let next = lines[i + 1];
                    // If next line is indented more, it's part of the expression body
                    let next_indent = next.len() - next.trim_start().len();
                    let curr_indent = line.len() - trimmed.len();
                    if next_indent > curr_indent && !next.trim().starts_with("//") {
                        // Multi-line expression body without braces
                        violations.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: self.id().to_string(),
                            message:
                                "Multi-line expression body should use braces or be on one line"
                                    .to_string(),
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
        NoSingleExpressionBody.check(&tree, source)
    }

    #[test]
    fn single_line_expression_ok() {
        assert!(check("fun foo() = 42\n").is_empty());
    }

    #[test]
    fn block_body_ok() {
        assert!(check("fun foo() {\n    return 42\n}\n").is_empty());
    }

    #[test]
    fn multiline_expression_no_braces() {
        let v = check("fun foo() =\n    1 + 2 + 3\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:no-single-expression-body");
    }
}
