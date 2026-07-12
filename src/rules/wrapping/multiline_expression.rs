//! standard:multiline-expression-wrapping — JVM ktlint parity.
//! Flag any multiline expression where content starts on same line as container.
use crate::rules::{Rule, Violation};

pub struct MultilineExpressionWrapping;
impl Rule for MultilineExpressionWrapping {
    fn id(&self) -> &'static str {
        "standard:multiline-expression-wrapping"
    }
    fn check(&self, tree: &tree_sitter::Tree, _source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        Self::walk(tree.root_node(), &mut v);
        v
    }
}
impl MultilineExpressionWrapping {
    fn walk(node: tree_sitter::Node, v: &mut Vec<Violation>) {
        let kind = node.kind();
        // Check ALL expression/block types that can span lines
        let is_expr = matches!(
            kind,
            "when_expression"
                | "when_entry"
                | "if_expression"
                | "binary_expression"
                | "call_expression"
                | "dot_qualified_expression"
                | "try_expression"
                | "comparison_expression"
                | "additive_expression"
                | "multiplicative_expression"
                | "elvis_expression"
                | "lambda_literal"
                | "function_body"
                | "class_body"
                | "is_expression"
                | "prefix_expression"
                | "postfix_expression"
                | "as_expression"
                | "return_expression"
                | "property_delegate"
                | "assignment"
                | "value_argument_list"
                | "function_value_parameters"
                | "class_parameters"
        );
        if is_expr {
            Self::check_node(&node, v);
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                Self::walk(c, v);
            }
        }
    }
    fn check_node(node: &tree_sitter::Node, v: &mut Vec<Violation>) {
        let sr = node.start_position().row;
        let er = node.end_position().row;
        if sr >= er {
            return;
        } // single-line
          // Check if ANY direct child is on same line as opening AND another child is on different line
        let mut has_same_line = false;
        let mut has_diff_line = false;
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                let cr = c.start_position().row;
                if cr == sr && c.kind() != "\n" && c.kind() != "(" && c.kind() != "{" {
                    has_same_line = true;
                    if i + 1 < node.child_count() {
                        if let Some(n) = node.child(i + 1) {
                            if n.start_position().row > sr {
                                has_diff_line = true;
                            }
                        }
                    }
                }
            }
        }
        if has_same_line && has_diff_line {
            v.push(Violation {
                file: String::new(),
                line: sr + 1,
                col: 1,
                rule_id: "standard:multiline-expression-wrapping".into(),
                auto_fixable: true,
                message: "A multiline expression should start on a new line".into(),
            });
        }
    }
}
