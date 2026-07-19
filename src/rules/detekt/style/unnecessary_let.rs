//! detekt:style:UnnecessaryLet — .let { it.xxx } can be just .xxx
use crate::rules::{Rule, Violation};
use tree_sitter::Node;

pub struct UnnecessaryLet;

impl Rule for UnnecessaryLet {
    fn id(&self) -> &'static str {
        "detekt:style:UnnecessaryLet"
    }
    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(n) = stack.pop() {
            if n.kind() == "call_expression" {
                check_call(&n, bytes, &mut v);
            }
            for i in (0..n.child_count()).rev() {
                if let Some(c) = n.child(i) {
                    stack.push(c);
                }
            }
        }
        v
    }
}

fn check_call(n: &Node, bytes: &[u8], v: &mut Vec<Violation>) {
    // Find navigation_expression child ending in .let
    let mut nav = None;
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            if c.kind() == "navigation_expression" {
                nav = Some(c);
                break;
            }
        }
    }
    let nav_node = match nav {
        Some(nn) => nn,
        None => return,
    };
    let nav_text = nav_node.utf8_text(bytes).unwrap_or("");
    if !nav_text.ends_with(".let") {
        return;
    }

    // Find the lambda literal body
    let mut lambda = None;
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            if c.kind() == "call_suffix" {
                lambda = find_lambda_body(&c);
                break;
            }
        }
    }
    let body = match lambda {
        Some(b) => b,
        None => return,
    };
    let body_text = body.utf8_text(bytes).unwrap_or("").trim().to_string();
    let body_text = body_text
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim()
        .to_string();

    if body_text == "it" || body_text.starts_with("it.") {
        let pos = n.start_position();
        v.push(Violation {
            file: String::new(),
            line: pos.row + 1,
            col: pos.column + 1,
            rule_id: "detekt:style:UnnecessaryLet".into(),
            message: ".let { it.xxx } can be simplified to just .xxx".into(),
            auto_fixable: false,
        });
    }
}

fn find_lambda_body<'a>(n: &Node<'a>) -> Option<Node<'a>> {
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            match c.kind() {
                "lambda_literal" => return Some(c),
                "annotated_lambda" | "call_suffix" => {
                    if let Some(found) = find_lambda_body(&c) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        UnnecessaryLet.check(&KotlinParser::new().parse(s), s)
    }
    #[test]
    fn let_it_prop_bad() {
        assert!(!c("val x = foo.let { it.bar }\n").is_empty());
    }
    #[test]
    fn let_it_call_bad() {
        assert!(!c("val x = foo.let { it.bar() }\n").is_empty());
    }
    #[test]
    fn let_custom_ok() {
        assert!(c("val x = bar.let { baz(it) }\n").is_empty());
    }
    #[test]
    fn no_let_ok() {
        assert!(c("val x = foo.bar()\n").is_empty());
    }
}
