//! standard:no-blank-line-in-list — no blank lines inside argument/parameter lists.
use crate::rules::{Rule, Violation};

pub struct NoBlankLineInList;
impl Rule for NoBlankLineInList {
    fn id(&self) -> &'static str { "standard:no-blank-line-in-list" }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        Self::walk(tree.root_node(), bytes, &mut v);
        v
    }
}
impl NoBlankLineInList {
    fn walk(node: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
        let k = node.kind();
        if k == "value_arguments" || k == "function_value_parameters" || k == "class_parameters" {
            let sr = node.start_position().row;
            let er = node.end_position().row;
            if er > sr + 1 { // multiline list
                // Check for blank lines within
                let mut last_content_row = sr;
                for i in 0..node.child_count() {
                    if let Some(c) = node.child(i) {
                        let cr = c.start_position().row;
                        if cr > last_content_row + 1 {
                            v.push(Violation { file: String::new(), line: cr, col: 1,
                                rule_id: "standard:no-blank-line-in-list".into(), auto_fixable: true,
                                message: "Unexpected blank line in argument/parameter list".into(),
                            });
                        }
                        if cr > last_content_row { last_content_row = cr; }
                    }
                }
            }
        }
        for i in 0..node.child_count() { if let Some(c) = node.child(i) { Self::walk(c, bytes, v); } }
    }
}
#[cfg(test)] mod tests { use super::*; use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> { let mut p=KotlinParser::new(); NoBlankLineInList.check(&p.parse(s), s) }
    #[test] fn ok() { assert!(c("fun foo(a: Int,\n    b: String\n)\n").is_empty()); }
    #[test] fn blank() { assert!(!c("fun foo(a: Int,\n\n    b: String\n)\n").is_empty()); }
}
