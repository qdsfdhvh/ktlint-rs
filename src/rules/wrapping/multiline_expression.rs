//! standard:multiline-expression-wrapping — JVM ktlint parity.
//!
//! Oracle semantics (verified against ktlint 1.8.0): when the right-hand
//! side of an `=` (property initializer, expression-body function,
//! parameter default value) spans multiple lines, the first token of that
//! expression must start on a new line — i.e. it must not share the line
//! with `=`:
//!
//! ```text
//! val x = foo(     // reported at `foo` (4:13)
//!     1,
//! )
//! val x =          // OK
//!     foo(1)
//! fun f() = foo(   // reported at `foo` (3:11)
//!     1,
//! )
//! ```
//!
//! `return` statements and plain call statements are exempt; only the
//! top-level RHS expression is checked (nested multiline expressions inside
//! a qualifying RHS are not reported separately).

use crate::rules::{Rule, Violation};

pub struct MultilineExpressionWrapping;

impl Rule for MultilineExpressionWrapping {
    fn id(&self) -> &'static str {
        "standard:multiline-expression-wrapping"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let bytes = source.as_bytes();
        let mut v = Vec::new();
        let mut stack: Vec<tree_sitter::Node> = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            match node.kind() {
                // property initializer: val x = <rhs>
                // parameter default value: a: Int = <rhs> (the `=` lives on
                //   function_value_parameters, next to the parameter)
                "property_declaration" | "function_value_parameters" => {
                    check_rhs(&node, bytes, &mut v);
                }
                // expression-body function: fun f() = <rhs> — the `=`
                // appears as the first child of its function_body
                "function_body" => {
                    check_eq_first(&node, bytes, &mut v);
                }
                _ => {}
            }
            for i in (0..node.child_count()).rev() {
                if let Some(c) = node.child(i) {
                    stack.push(c);
                }
            }
        }
        v
    }
}

/// Scan siblings for `=` followed by the first named expression node.
fn check_rhs(node: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    let mut after_eq = false;
    for c in node.children(&mut node.walk()) {
        if c.kind() == "=" {
            after_eq = true;
            continue;
        }
        if after_eq && c.is_named() {
            // A lambda literal directly assigned (`val f = { x ->`) is
            // exempt — ktlint does not demand the `{` move (oracle).
            if c.kind() == "lambda_literal" {
                return;
            }
            report_if_multiline_share_line(&c, bytes, violations);
            return;
        }
    }
}

/// function_body whose first named child follows a leading `=` —
/// expression-body function. Exempt when the function signature itself
/// spans multiple lines (oracle: `fun f(\n    a: Int,\n) = foo(\n    1,\n)`
/// is not reported — the `=` then sits on the `)` line).
fn check_eq_first(node: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    if let Some(parent) = node.parent() {
        let sig_multiline = parent.children(&mut parent.walk()).any(|c| {
            c.kind() == "function_value_parameters"
                && c.start_position().row != c.end_position().row
        });
        if sig_multiline {
            return;
        }
    }
    let mut after_eq = false;
    for c in node.children(&mut node.walk()) {
        if c.kind() == "=" {
            after_eq = true;
            continue;
        }
        if after_eq && c.is_named() {
            report_if_multiline_share_line(&c, bytes, violations);
            return;
        }
        if c.kind() == "{" {
            return; // block body — not this rule
        }
    }
}

/// Report when the expression spans multiple lines but its first token is
/// not the first token of its line (it shares the line with `=`).
fn report_if_multiline_share_line(
    rhs: &tree_sitter::Node,
    bytes: &[u8],
    violations: &mut Vec<Violation>,
) {
    if rhs.start_position().row == rhs.end_position().row {
        return; // single-line RHS is fine
    }
    let start = rhs.start_byte();
    let precedes_ws_only = bytes[..start]
        .iter()
        .rev()
        .take_while(|&&b| b != b'\n')
        .all(|&b| b == b' ' || b == b'\t');
    if precedes_ws_only {
        return; // starts a fresh line — OK
    }
    violations.push(Violation {
        file: String::new(),
        line: rhs.start_position().row + 1,
        col: rhs.start_position().column + 1,
        rule_id: "standard:multiline-expression-wrapping".into(),
        message: "A multiline expression should start on a new line".into(),
        auto_fixable: true,
    });
}
