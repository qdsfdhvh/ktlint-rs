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
            if trimmed.starts_with("fun ") && trimmed.contains('=') && !trimmed.contains('{') {
                // Issue #45: skip trailing lambda — check if next line starts with { or .
                if i + 1 < lines.len() {
                    let next = lines[i + 1];
                    let next_indent = next.len() - next.trim_start().len();
                    let curr_indent = line.len() - trimmed.len();
                    let next_trim = next.trim();
                    let is_trailing_lambda = next_trim.starts_with('{')
                        || next_trim.starts_with('.');
                    if next_indent > curr_indent
                        && !next_trim.starts_with("//")
                        && !is_trailing_lambda
                    {
                        violations.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: self.id().to_string(),
                            message: "Multi-line expression body should use braces or be on one line"
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

    // ── Issue #45: Trailing lambda ──

    #[test]
    fn trailing_lambda_expression_ok() {
        let src = "fun render() = wrapper {\n    consume(1f)\n}\n";
        let v = check(src);
        assert!(v.is_empty(), "trailing lambda should not be flagged, got {:?}", v);
    }

    #[test]
    fn chained_call_expression_ok() {
        let src = "fun foo() = bar\n    .baz()\n";
        let v = check(src);
        assert!(v.is_empty(), "chained call should not be flagged, got {:?}", v);
    }
}
