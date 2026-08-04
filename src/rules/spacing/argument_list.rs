//! `standard:argument-list-wrapping` — wrap an argument/parameter list when it
//! exceeds the max line length or already contains a newline. Mirrors ktlint
//! 1.8.0 (ArgumentListWrappingRule):
//! - if the single-line list exceeds max_line_length, each argument goes on its
//!   own line (unless it cannot be split — lambda arguments are skipped)
//! - if the list already spans lines, each argument must be on its own line
//! - `(` stays on the callee line; `)` goes on its own line aligned with the
//!   opening line

use crate::rules::{Rule, Violation};

pub struct ArgumentListWrapping;

impl Rule for ArgumentListWrapping {
    fn id(&self) -> &'static str {
        "standard:argument-list-wrapping"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if matches!(
                node.kind(),
                "value_arguments" | "class_parameters" | "function_value_parameters"
            ) {
                self.check_list(&node, bytes, source, &mut violations);
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

impl ArgumentListWrapping {
    fn check_list(
        &self,
        node: &tree_sitter::Node,
        bytes: &[u8],
        source: &str,
        violations: &mut Vec<Violation>,
    ) {
        let start_row = node.start_position().row;
        let end_row = node.end_position().row;
        let max_line_length = 120; // default; configurable via .editorconfig

        // Collect the argument/parameter children (skip `(`, `)`, commas,
        // newlines, comments).
        let args: Vec<tree_sitter::Node> = (0..node.child_count())
            .filter_map(|i| node.child(i))
            .filter(|c| {
                !matches!(
                    c.kind(),
                    "(" | ")" | "," | "\n" | "comment" | "multiline_comment"
                )
            })
            .collect();
        if args.is_empty() {
            return;
        }

        // Skip lambda arguments: `foo { ... }` — wrapping a lambda is not useful.
        let has_lambda = args.iter().any(|a| {
            let text = &source[a.start_byte()..a.end_byte()];
            text.trim_start().starts_with("{") || text.trim_end().ends_with("}")
        });

        // Determine if wrapping is needed: list already spans lines, or the
        // single-line list exceeds max_line_length. ktlint measures the whole
        // line (leavesOnLine20), not just the list node.
        let single_line = start_row == end_row;
        let line_len = if single_line {
            let line_start = source[..node.start_byte()].rfind('\n').map_or(0, |i| i + 1);
            let line_end = source[node.end_byte()..]
                .find('\n')
                .map_or(source.len(), |i| node.end_byte() + i);
            line_end - line_start
        } else {
            0
        };
        let exceeds = line_len > max_line_length;
        let need_wrap = (single_line && exceeds && !has_lambda) || (!single_line && !has_lambda);

        if !need_wrap {
            return;
        }

        if !single_line {
            // Already multiline: each argument should be on its own line.
            let mut prev_row = start_row;
            for arg in &args {
                let row = arg.start_position().row;
                if row == prev_row && row != start_row && !arg.kind().contains("comment") {
                    violations.push(self.v(*arg, source));
                }
                prev_row = row;
            }
        } else {
            // Single-line exceeding limit: each argument should be wrapped.
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    violations.push(self.v(*arg, source));
                }
            }
        }
    }

    fn v(&self, node: tree_sitter::Node, source: &str) -> Violation {
        let start = node.start_byte();
        let line = source[..start].bytes().filter(|&b| b == b'\n').count() + 1;
        let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
        let col = start - line_start + 1;
        Violation {
            file: String::new(),
            line,
            col,
            rule_id: self.id().into(),
            message:
                "Argument should be on a separate line (unless all arguments can fit a single line)"
                    .into(),
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
        ArgumentListWrapping.check(&p.parse(s), s)
    }

    #[test]
    fn short_list_no_wrap() {
        assert!(c("val x = listOf(1, 2, 3)\n").is_empty());
    }

    #[test]
    fn long_list_wraps() {
        let src = "val x = listOf(\"aaaaaaaaaaaaaaaaaaaa\", \"bbbbbbbbbbbbbbbbbbbb\", \"cccccccccccccccccccc\", \"dddddddddddddddddddd\", \"eeeeeeeeeeeeeeeeeeee\", \"ffffffffffffffffffff\", \"gggggggggggggggggggg\")\n";
        assert!(!c(src).is_empty(), "long list should report wrapping");
    }

    #[test]
    fn lambda_skipped() {
        assert!(c("val x = listOf(1, 2).map { it * 2 }\n").is_empty());
    }
}
