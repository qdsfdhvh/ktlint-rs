//! standard:no-unnecessary-parentheses-before-trailing-lambda — remove parens on trailing lambda.
use crate::rules::{Rule, Violation};

pub struct LambdaParen;

impl Rule for LambdaParen {
    fn id(&self) -> &'static str {
        "standard:no-unnecessary-parentheses-before-trailing-lambda"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        if tree.root_node().has_error() {
            return Vec::new();
        }
        let mut violations = Vec::new();
        walk(tree.root_node(), source.as_bytes(), &mut violations);
        violations
    }
}

fn is_function_parameter_suffix(node: &tree_sitter::Node, bytes: &[u8]) -> bool {
    let line_start = bytes[..node.start_byte()]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |position| position + 1);
    let prefix = std::str::from_utf8(&bytes[line_start..node.start_byte()]).unwrap_or("");
    prefix
        .rsplit_once("fun ")
        .map(|(_, tail)| tail.trim())
        .is_some_and(|after_fun| {
            !after_fun.is_empty()
                && after_fun.chars().all(|ch| {
                    ch.is_alphanumeric() || matches!(ch, '_' | '`' | '.' | '<' | '>' | '?')
                })
        })
}

// Walk CST for call_expression/call_suffix nodes with empty value_arguments + trailing lambda
fn walk(node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    let kind = node.kind();

    // A `call_suffix` owns both the value arguments and its trailing lambda. Checking
    // the wider `call_expression` can combine empty parentheses from one descendant
    // with an unrelated nested lambda and report the enclosing declaration.
    if kind == "call_suffix"
        && node
            .parent()
            .is_some_and(|parent| parent.kind() == "call_expression")
        && !is_function_parameter_suffix(&node, bytes)
    {
        let mut found_parens = false;
        let mut found_lambda = false;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "value_arguments" => {
                        // Check if parens are empty
                        let mut empty = true;
                        for j in 0..child.child_count() {
                            if let Some(arg) = child.child(j) {
                                let k = arg.kind();
                                if k != "(" && k != ")" && k != "," {
                                    let text = arg.utf8_text(bytes).unwrap_or("");
                                    if !text.chars().all(|c| c.is_whitespace()) {
                                        empty = false;
                                    }
                                }
                            }
                        }
                        if empty {
                            found_parens = true;
                        }
                    }
                    // tree-sitter-kotlin-sg wraps lambdas in annotated_lambda
                    "annotated_lambda" | "lambda_literal" | "anonymous_function" => {
                        found_lambda = true;
                    }
                    _ => {}
                }
            }
        }

        let text = node.utf8_text(bytes).unwrap_or("").trim_start();
        let adjacent_empty_parens = text
            .strip_prefix("()")
            .is_some_and(|rest| rest.trim_start().starts_with('{'));
        if found_parens && found_lambda && adjacent_empty_parens {
            let pos = node.start_position();
            violations.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: "standard:no-unnecessary-parentheses-before-trailing-lambda".into(),
                message: "Unnecessary parentheses before trailing lambda".into(),
                auto_fixable: true,
            });
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, bytes, violations);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(src);
        LambdaParen.check(&tree, src)
    }

    #[test]
    fn trailing_lambda_with_empty_parens_is_bad() {
        assert!(!check("fun f() { list.forEach() { it } }\n").is_empty());
    }

    #[test]
    fn trailing_lambda_without_parens_is_good() {
        assert!(check("fun f() { list.forEach { it } }\n").is_empty());
    }

    #[test]
    fn function_declaration_not_flagged() {
        assert!(check("fun foo() { println(\"hi\") }\n").is_empty());
    }

    #[test]
    fn call_with_args_not_flagged() {
        assert!(check("fun f() { foo(1) { it } }\n").is_empty());
    }

    #[test]
    fn nested_lambda_does_not_pair_with_unrelated_empty_call() {
        let source = "fun render() {\n    host {\n        value.toString()\n    }\n}\n";
        assert!(check(source).is_empty());
    }

    #[test]
    fn run_apply_also_takeIf_takeUnless() {
        assert!(!check("fun f() { x.run() { it } }\n").is_empty());
        assert!(!check("fun f() { x.apply() { } }\n").is_empty());
        assert!(!check("fun f() { x.also() { } }\n").is_empty());
        assert!(!check("fun f() { x.takeIf() { it } }\n").is_empty());
        assert!(!check("fun f() { x.takeUnless() { it } }\n").is_empty());
    }
}
