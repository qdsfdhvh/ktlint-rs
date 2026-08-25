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
            // Only call-site argument lists (the parent is a call) —
            // class/function parameter lists and function types
            // (`(() -> Unit)?`) are not this rule's territory (oracle).
            if matches!(node.kind(), "value_arguments")
                && node
                    .parent()
                    .is_some_and(|p| matches!(p.kind(), "call_expression" | "call_suffix"))
                // A function-TYPE paren mis-parsed as a call's value_arguments
                // (`onDismissStarted: (() -> Unit)? = null`, `@Composable
                // (Type) -> Unit`) is not an argument list — the opener's
                // leading token is `@`, `name:`, or a non-identifier
                // (kataris corpus, oracle clean).
                && !type_paren_arguments(&node, bytes)
            {
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
        _bytes: &[u8],
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
                ) && c.kind() != "parameter_modifiers"
            })
            .collect();
        if args.is_empty() {
            return;
        }
        // ktlint skips lists with >= 8 arguments
        // (ktlint_argument_list_wrapping_ignore_when_parameter_count_greater_
        // or_equal_than, default 8, oracle-probed).
        if args.len() > 8 {
            return;
        }

        // Skip lambda arguments: `foo { ... }` — wrapping a lambda is not useful.
        let has_lambda = args.iter().any(|a| {
            let text = &source[a.start_byte()..a.end_byte()];
            text.trim_start().starts_with("{") || text.trim_end().ends_with("}")
        });

        // Determine if wrapping is needed: list already spans lines, or the
        // single-line list exceeds max_line_length. ktlint measures the whole
        // physical line (leavesOnLine from the line's first leaf, oracle:
        // `fun f() = call(…long args…)` reports on the full line; an
        // assignment at class-member level is parsed differently and is not
        // covered here).
        let single_line = start_row == end_row;
        // ktlint measures the leaves from the list's opening `(` to the end
        // of the line (oracle: a 128-char 4-arg call whose `(` sits at col 24
        // is not reported — 104 chars from `(`; the same args with a longer
        // name reach 121 from `(` and are reported).
        let line_len = if single_line {
            let line_end = source[node.end_byte()..]
                .find('\n')
                .map_or(source.len(), |i| node.end_byte() + i);
            // ktlint measures the line in characters, not bytes — a CJK
            // argument (3 UTF-8 bytes per char) must not inflate the width
            // (kataris corpus: `previewStory("sto_1", "…40-char CJK…", 187000)`
            // is 250+ bytes but ~100 chars; ktlint 1.8 stays silent).
            source[node.start_byte()..line_end].chars().count()
        } else {
            0
        };
        let exceeds = line_len > max_line_length;
        let need_wrap = (single_line && exceeds && !has_lambda) || (!single_line && !has_lambda);

        if !need_wrap {
            return;
        }

        if !single_line {
            // Already multiline: each argument should be on its own line —
            // including the first when it sits on the opening-paren line
            // (`foo("a",\n    "b")` reports `"a"`, issue #204).
            let mut prev_row = start_row;
            for arg in &args {
                let row = arg.start_position().row;
                if row == prev_row && !arg.kind().contains("comment") {
                    violations.push(self.v(*arg, source));
                }
                prev_row = row;
            }
            // A closing paren sharing the last argument's line
            // (`"b")`) draws "Missing newline before \")\"".
            if let (Some(last), Some(rp)) = (
                args.last(),
                node.children(&mut node.walk()).find(|c| c.kind() == ")"),
            ) {
                if rp.start_position().row == last.end_position().row {
                    let pos = rp.start_position();
                    violations.push(Violation {
                        file: String::new(),
                        line: pos.row + 1,
                        col: pos.column + 1,
                        rule_id: self.id().to_string(),
                        message: "Missing newline before \")\"".to_string(),
                        auto_fixable: true,
                    });
                }
            }
        } else {
            // Single-line exceeding limit: each argument should be wrapped,
            // including the first (ktlint reports the first argument too).
            for arg in &args {
                violations.push(self.v(*arg, source));
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

#[cfg(test)]
mod argument_list_cjk_width_tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(src);
        ArgumentListWrapping.check(&tree, src)
    }

    // Issue #260: a CJK argument is 3 UTF-8 bytes per char — the width must
    // be counted in chars so a ~100-char line with 40 CJK chars does not
    // look 250 bytes wide (oracle 1.8 stays silent).
    #[test]
    fn cjk_argument_not_byte_inflated() {
        let src = "package com.example\n\nfun preview() {\n    previewStory(\"sto_1\", \"身代わり婚のはずが、冷酷な黒竜様の狂おしい執着愛から逃げられません\", 187000)\n    use()\n}\n";
        assert!(check(src).is_empty());
    }
}

/// True when a mis-parsed `value_arguments` is actually a function TYPE
/// paren: the opening `(` is preceded by `@Name`, `name:`, or a
/// non-identifier character (type/grouping context) — not a call's callee.
/// tree-sitter-kotlin-sg produces these for `onDismissStarted: (() -> Unit)?`
/// and `content: @Composable (Type) -> Unit` parameter types (kataris
/// corpus, oracle clean — argument-list-wrapping must not fire).
fn type_paren_arguments(node: &tree_sitter::Node, bytes: &[u8]) -> bool {
    let Some(open) = node.children(&mut node.walk()).find(|c| c.kind() == "(") else {
        return false;
    };
    let start = open.start_byte();
    if start == 0 || bytes[start - 1] != b' ' {
        return false;
    }
    let mut j = start - 1;
    while j > 0 && (bytes[j - 1] == b' ' || bytes[j - 1] == b'\t') {
        j -= 1;
    }
    let mut k = j;
    while k > 0
        && (bytes[k - 1].is_ascii_alphanumeric() || bytes[k - 1] == b'_' || bytes[k - 1] == b'.')
    {
        k -= 1;
    }
    if k > 0 && (bytes[k - 1] == b'@' || bytes[k - 1] == b':') {
        return true;
    }
    // A non-identifier char before the space run (`= (`, `, (`, `((`).
    j > 0 && !bytes[j - 1].is_ascii_alphanumeric() && bytes[j - 1] != b'_'
}

#[cfg(test)]
mod argument_list_type_paren_tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(src);
        ArgumentListWrapping.check(&tree, src)
    }

    // Function-TYPE parameter parens mis-parsed as value_arguments
    // (`onDismissStarted: (() -> Unit)? = null`) are not argument lists
    // (oracle clean, kataris KatBottomSheet).
    #[test]
    fn fn_type_params_not_argument_lists() {
        let src = "package com.example\n\nprivate fun sheet(\n    onDismissStarted: (() -> Unit)? = null,\n    content: @Composable KatBottomSheetScope.() -> Unit,\n) {\n    use(onDismissStarted, content)\n}\n";
        assert!(check(src).is_empty());
    }
}
