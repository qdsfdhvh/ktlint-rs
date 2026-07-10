//! standard:no-blank-line-before-list-closing — no blank before ) in multiline lists.
use crate::rules::{Rule, Violation};

pub struct NoBlankBeforeListClose;
impl Rule for NoBlankBeforeListClose {
    fn id(&self) -> &'static str {
        "standard:no-blank-line-before-list-closing"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        Self::walk(tree.root_node(), source, &mut v);
        v
    }
}
impl NoBlankBeforeListClose {
    fn walk(node: tree_sitter::Node, source: &str, v: &mut Vec<Violation>) {
        let k = node.kind();
        if k == "value_arguments" || k == "function_value_parameters" {
            let c = node.child_count();
            if c > 0 {
                if let Some(last) = node.child(c - 1) {
                    if last.kind() == ")" {
                        let lr = last.start_position().row;
                        // Check previous content row
                        for i in (0..c - 1).rev() {
                            if let Some(prev) = node.child(i) {
                                let pr = prev.start_position().row;
                                if prev.kind() != "(" && prev.kind() != "comment" {
                                    if pr + 1 < lr {
                                        v.push(Violation {
                                            file: String::new(),
                                            line: lr + 1,
                                            col: 1,
                                            rule_id: "standard:no-blank-line-before-list-closing"
                                                .into(),
                                            auto_fixable: true,
                                            message: "Unexpected blank line before closing \")\""
                                                .into(),
                                        });
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                Self::walk(c, source, v);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        NoBlankBeforeListClose.check(&p.parse(s), s)
    }
    #[test]
    fn ok() {
        assert!(c("fun f(a: Int,\n      b: String\n)\n").is_empty());
    }
    #[test]
    fn bad() {
        assert!(!c("fun f(a: Int,\n\n)\n").is_empty());
    }
}
