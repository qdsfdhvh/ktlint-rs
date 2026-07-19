//! standard:no-empty-first-line-in-class-body
//! AST-based: walks CST for class_body nodes, checks blank line after opening brace.

use crate::rules::{Rule, Violation};

pub struct NoEmptyFirstLineInClassBody;

/// CST node types that have a body with braces.
const BODY_TYPES: &[&str] = &[
    "class_body", "object_body", "enum_class_body", "companion_object"
];

impl Rule for NoEmptyFirstLineInClassBody {
    fn id(&self) -> &'static str {
        "standard:no-empty-first-line-in-class-body"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        walk(tree.root_node(), bytes, source, &mut v);
        v
    }
}

fn walk(root: tree_sitter::Node, bytes: &[u8], source: &str, v: &mut Vec<Violation>) {
    let mut stack: Vec<tree_sitter::Node> = vec![root];
    while let Some(node) = stack.pop() {
        if BODY_TYPES.contains(&node.kind()) {
            check_body(&node, bytes, source, v);
        }
        for i in (0..node.child_count()).rev() {
            if let Some(c) = node.child(i) { stack.push(c); }
        }
    }
}

fn check_body(body: &tree_sitter::Node, bytes: &[u8], source: &str, v: &mut Vec<Violation>) {
    // Find the opening { in this body node
    let body_text = body.utf8_text(bytes).unwrap_or("");
    let brace_pos = match body_text.find('{') {
        Some(p) => p,
        None => return,
    };
    // Text after {
    let after_brace = &body_text[brace_pos + 1..];
    // Check if first non-whitespace is preceded by a blank line (\\n\\n after {)
    let has_blank = after_brace.starts_with("\n\n")
        || after_brace.starts_with("\r\n\r\n")
        || (after_brace.starts_with('\n') && after_brace[1..].trim_start().starts_with('\n'));
    if !has_blank { return; }

    // Find line number: offset of the opening { in source
    let brace_byte = body.start_byte() + brace_pos;
    let brace_line = source[..brace_byte].chars().filter(|&c| c == '\n').count() + 1;

    v.push(Violation {
        file: String::new(),
        line: brace_line + 1,
        col: 1,
        rule_id: "standard:no-empty-first-line-in-class-body".into(),
        message: "Unexpected blank line in class body".into(),
        auto_fixable: true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        NoEmptyFirstLineInClassBody.check(&KotlinParser::new().parse(s), s)
    }
    #[test] fn ok() { assert!(c("class Foo {\n    val x=1\n}\n").is_empty()); }
    #[test] fn bad() { assert!(!c("class Foo {\n\n    val x=1\n}\n").is_empty()); }
    #[test] fn fun_ignored() { assert!(c("fun bar() {\n    return 1\n}\n").is_empty()); }
    #[test] fn data_class_bad() { assert!(!c("data class Foo {\n\n    val x=1\n}\n").is_empty()); }
    #[test] fn enum_class_bad() { assert!(!c("enum class Foo {\n\n    A\n}\n").is_empty()); }
    #[test] fn sealed_class_bad() { assert!(!c("sealed class Foo {\n\n    class A: Foo()\n}\n").is_empty()); }
    #[test] fn companion_object_bad() { assert!(!c("class Foo {\n    companion object {\n\n        val x=1\n    }\n}\n").is_empty()); }
    #[test] fn interface_bad() { assert!(!c("interface Foo {\n\n    fun bar()\n}\n").is_empty()); }
    #[test] fn comments_no_flag() { assert!(c("class Foo {\n    // comment\n    val x=1\n}\n").is_empty()); }
}
