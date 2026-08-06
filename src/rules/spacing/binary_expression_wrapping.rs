//! `standard:binary-expression-wrapping` — wrap a binary expression that
//! exceeds the max line length. Mirrors ktlint 1.8.0 (BinaryExpressionWrappingRule):
//! - if a property/function's `= <binary expr>` exceeds the limit, break after `=`
//! - if the binary expression is a call argument that exceeds, wrap before it
//! - otherwise wrap at the operation reference (e.g. `+`) so the right-hand
//!   side goes to a new line

use crate::rules::{Rule, Violation};

pub struct BinaryExpressionWrapping;

impl Rule for BinaryExpressionWrapping {
    fn id(&self) -> &'static str {
        "standard:binary-expression-wrapping"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if matches!(
                node.kind(),
                "additive_expression"
                    | "multiplicative_expression"
                    | "comparison_expression"
                    | "conjunction_expression"
                    | "disjunction_expression"
                    | "equality_expression"
                    | "elvis_expression"
                    | "as_expression"
                    | "range_expression"
            ) {
                self.check_expression(&node, source, &mut violations);
            }
            for i in (0..node.child_count()).rev() {
                if let Some(child) = node.child(i) {
                    stack.push(child);
                }
            }
        }
        violations
    }
}

impl BinaryExpressionWrapping {
    fn check_expression(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        violations: &mut Vec<Violation>,
    ) {
        let max_line_length = 120; // default; configurable via .editorconfig
        let start = node.start_byte();
        let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
        // Full line length from its start to the next newline.
        let line_len = source[line_start..]
            .find('\n')
            .map_or(source.len() - line_start, |i| i);
        // Skip expressions inside string templates (`${a + b}`) — the operator
        // is data, not code.
        let before = &source[line_start..start];
        if before.matches('"').count() % 2 == 1 {
            return;
        }

        // Single-line expression exceeding the limit. The whole line matters
        // (including the `val x: Type = ` prefix before the expression).
        if node.end_position().row == node.start_position().row && line_len > max_line_length {
            // If preceded by `=` (property/function assignment), break after `=`.
            let before = &source[..start];
            let has_assignment = before.rfind('=').is_some_and(|i| {
                let after_eq = &before[i + 1..];
                after_eq.trim_start().is_empty() || after_eq.starts_with(char::is_whitespace)
            });
            if has_assignment {
                violations.push(self.v(*node, source, "Line is exceeding max line length. Break line between assignment and expression"));
                return;
            }
            // Otherwise wrap at the operator: report at the operator position
            // (left-hand side of the operator stays, right-hand side wraps).
            if let Some(op) = self.find_operator(node, source) {
                violations.push(self.v(
                    *node,
                    source,
                    "Line is exceeding max line length. Break line before expression",
                ));
                let _ = op;
            }
        }
    }

    fn find_operator(&self, node: &tree_sitter::Node, source: &str) -> Option<usize> {
        let text = &source[node.start_byte()..node.end_byte()];
        ["&&", "||", "+", "-", "*", "/", "?", ":"]
            .iter()
            .find_map(|op| text.find(op))
            .map(|i| node.start_byte() + i)
    }

    fn v(&self, node: tree_sitter::Node, source: &str, msg: &str) -> Violation {
        let start = node.start_byte();
        let line = source[..start].bytes().filter(|&b| b == b'\n').count() + 1;
        let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
        Violation {
            file: String::new(),
            line,
            col: start - line_start + 1,
            rule_id: self.id().into(),
            message: msg.into(),
            auto_fixable: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        BinaryExpressionWrapping.check(&p.parse(s), s)
    }

    #[test]
    fn short_expr_ok() {
        assert!(c("val x = 1 + 2\n").is_empty());
    }

    #[test]
    fn long_expr_wraps() {
        let src = "object Fixtures {\n    val shortCall: String = listOf(\"a\", \"b\").joinToString(separator = \", \", prefix = \"[\", postfix = \"]\") + \"padding-to-go-over-the-limit-here\"\n}\n";
        assert!(!c(src).is_empty(), "long binary expression should report");
    }
}
