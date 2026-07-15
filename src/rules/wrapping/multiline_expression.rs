//! standard:multiline-expression-wrapping — JVM ktlint parity.
//! Only flags when/if/try/function/class bodies where content starts on same line as `{`.

use crate::rules::{Rule, Violation};

pub struct MultilineExpressionWrapping;

impl Rule for MultilineExpressionWrapping {
    fn id(&self) -> &'static str { "standard:multiline-expression-wrapping" }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk(tree.root_node(), source.as_bytes(), &mut v);
        v
    }
}

fn walk(root: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    let mut stack: Vec<tree_sitter::Node> = vec![root];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        let is_target = matches!(
            kind,
            "when_expression" | "if_expression" | "try_expression"
                | "function_declaration" | "class_declaration"
        );
        if is_target {
            check_node(&node, bytes, violations);
        }
        for i in (0..node.child_count()).rev() {
            if let Some(c) = node.child(i) { stack.push(c); }
        }
    }
}

fn check_node(node: &tree_sitter::Node, _bytes: &[u8], violations: &mut Vec<Violation>) {
    let sr = node.start_position().row;
    let er = node.end_position().row;
    if sr >= er { return; }

    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        let ck = child.kind();

        if ck == "function_body" || ck == "class_body" || ck == "statements" {
            let body_row = child.start_position().row;
            let mut saw_open = false;
            for j in 0..child.child_count() {
                let Some(bc) = child.child(j) else { continue };
                if bc.kind() == "{" { saw_open = true; continue; }
                if saw_open && !bc.is_extra() {
                    if bc.start_position().row == body_row {
                        violations.push(Violation {
                            file: String::new(), line: body_row + 1, col: 1,
                            rule_id: "standard:multiline-expression-wrapping".into(),
                            auto_fixable: true,
                            message: "A multiline expression should start on a new line".into(),
                        });
                    }
                    break;
                }
            }
        }

        if ck == "{" {
            let open_row = child.start_position().row;
            for n in (i + 1)..node.child_count() {
                let Some(nc) = node.child(n) else { continue };
                if !nc.is_extra() && nc.kind() != "}" {
                    if nc.start_position().row == open_row {
                        violations.push(Violation {
                            file: String::new(), line: open_row + 1, col: 1,
                            rule_id: "standard:multiline-expression-wrapping".into(),
                            auto_fixable: true,
                            message: "A multiline expression should start on a new line".into(),
                        });
                    }
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn check(src: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        MultilineExpressionWrapping.check(&p.parse(src), src)
    }
    #[test] fn content_on_newline_ok() { assert!(check("fun foo() {\n    return 1\n}\n").is_empty()); }
    #[test] fn content_same_line_bad() { assert!(!check("fun foo() { return 1\n}\n").is_empty()); }
    #[test] fn when_multiline_ok() { assert!(check("when (x) {\n    1 -> a()\n    2 -> b()\n}\n").is_empty()); }
    #[test] fn when_content_same_line_bad() { assert!(!check("when (x) { 1 -> a()\n    2 -> b()\n}\n").is_empty()); }
    #[test] fn if_multiline_ok() { assert!(check("if (x) {\n    a()\n}\n").is_empty()); }
    #[test] fn inline_call_expression_not_flagged() { assert!(check("fun foo() {\n    bar(\n        x,\n        y\n    )\n}\n").is_empty()); }
    #[test] fn binary_expression_not_flagged() { assert!(check("val x = a +\n    b\n").is_empty()); }
    #[test] fn dot_qualified_not_flagged() { assert!(check("val x = obj\n    .method()\n").is_empty()); }
}
