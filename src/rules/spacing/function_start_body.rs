//! standard:function-start-of-body-spacing — the whitespace around the start
//! of a function body (`=` for expression bodies, `{` for block bodies) must
//! be exactly one space (or a line break).

use crate::rules::{Rule, Violation};

pub struct FunctionStartOfBodySpacing;

impl Rule for FunctionStartOfBodySpacing {
    fn id(&self) -> &'static str {
        "standard:function-start-of-body-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "function_declaration" {
                check_function(node, source, &mut violations);
            }
            let mut w = node.walk();
            let mut kids = Vec::new();
            for c in node.children(&mut w) {
                kids.push(c);
            }
            for c in kids.into_iter().rev() {
                stack.push(c);
            }
        }
        violations
    }
}

fn check_function(func: tree_sitter::Node, source: &str, out: &mut Vec<Violation>) {
    let start = func.start_byte();
    let end = func.end_byte();
    // Find the function body (its text starts with `=` or `{`).
    let mut w = func.walk();
    let children: Vec<tree_sitter::Node> = func.children(&mut w).collect();
    let body = children.iter().find(|c| c.kind() == "function_body");
    let Some(body) = body else { return };

    let body_text = &source[body.start_byte()..body.end_byte()];
    if let Some(rest) = body_text.strip_prefix('=') {
        // Expression body: check whitespace before and after `=`.
        let eq_abs = body.start_byte();
        // Whitespace before `=`.
        let before_start = source[..eq_abs].rfind('\n').map_or(start, |i| i + 1);
        let before = &source[before_start..eq_abs];
        // Trailing spaces directly before `=` (the rest of the line is the
        // signature). Exactly one is required.
        let spaces = before.len() - before.trim_end().len();
        if spaces != 1 {
            // ktlint reports at the first space: `fun a(): Int  = 1` → the
            // column of the `=` minus the number of spaces.
            let line = source[..eq_abs].bytes().filter(|&b| b == b'\n').count() + 1;
            let col = eq_abs - source[..eq_abs].rfind('\n').map_or(0, |i| i + 1) - spaces;
            out.push(Violation {
                file: String::new(),
                line,
                col: col + 1,
                rule_id: "standard:function-start-of-body-spacing".into(),
                message: "Unexpected whitespace".into(),
                auto_fixable: true,
            });
        }
        // Whitespace after `=` (same line only — a line break is fine).
        let after = rest;
        if let Some(first) = after.chars().next() {
            if first == ' ' {
                let spaces_after = after.bytes().take_while(|b| *b == b' ').count();
                if spaces_after != 1 {
                    // `=  1` → report at the second space (col of `=` + 2).
                    let line = source[..eq_abs].bytes().filter(|&b| b == b'\n').count() + 1;
                    let eq_col = eq_abs - source[..eq_abs].rfind('\n').map_or(0, |i| i + 1);
                    out.push(Violation {
                        file: String::new(),
                        line,
                        col: eq_col + 1,
                        rule_id: "standard:function-start-of-body-spacing".into(),
                        message:
                            "Expected a single white space between assignment and expression body on same line"
                                .into(),
                        auto_fixable: true,
                    });
                }
            }
        }
    } else if body_text.trim_start().starts_with('{') {
        // Block body: check whitespace before `{`.
        let lbrace = body_text.find('{').map(|r| body.start_byte() + r);
        let Some(lbrace) = lbrace else { return };
        let before_start = source[..lbrace].rfind('\n').map_or(start, |i| i + 1);
        let before = &source[before_start..lbrace];
        let spaces = before.len() - before.trim_end().len();
        if spaces != 1 {
            // `fun c()  {` → report at `{` (col of `{`).
            let line = source[..lbrace].bytes().filter(|&b| b == b'\n').count() + 1;
            let col = lbrace - source[..lbrace].rfind('\n').map_or(0, |i| i + 1);
            out.push(Violation {
                file: String::new(),
                line,
                col: col + 1,
                rule_id: "standard:function-start-of-body-spacing".into(),
                message: "Expected a single white space before start of function body".into(),
                auto_fixable: true,
            });
        }
    }
    let _ = end;
}
