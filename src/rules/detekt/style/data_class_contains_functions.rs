//! detekt:style:DataClassContainsFunctions — data classes should not contain functions
use crate::rules::{Rule, Violation};
use tree_sitter::Node;

pub struct DataClassContainsFunctions;

impl Rule for DataClassContainsFunctions {
    fn id(&self) -> &'static str {
        "detekt:style:DataClassContainsFunctions"
    }
    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(n) = stack.pop() {
            if n.kind() == "class_declaration" {
                check_data_class(&n, bytes, &mut v);
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

fn check_data_class(n: &Node, bytes: &[u8], v: &mut Vec<Violation>) {
    let text = n.utf8_text(bytes).unwrap_or("");
    if !text.starts_with("data ") {
        return;
    }

    // Find class_body, then count functions
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            if c.kind() == "class_body" {
                let mut fn_count = 0usize;
                for j in 0..c.child_count() {
                    if let Some(member) = c.child(j) {
                        if member.kind() == "function_declaration" {
                            fn_count += 1;
                        }
                    }
                }
                if fn_count > 0 {
                    let pos = n.start_position();
                    v.push(Violation {
                        file: String::new(),
                        line: pos.row + 1,
                        col: pos.column + 1,
                        rule_id: "detekt:style:DataClassContainsFunctions".into(),
                        message: format!(
                            "Data class contains {} function(s) — move to companion or extension",
                            fn_count
                        ),
                        auto_fixable: false,
                    });
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        DataClassContainsFunctions.check(&KotlinParser::new().parse(s), s)
    }
    #[test]
    fn data_class_with_fn_bad() {
        assert!(!c("data class Foo(val x: Int) { fun f() = x }\n").is_empty());
    }
    #[test]
    fn data_class_no_fn_ok() {
        assert!(c("data class Foo(val x: Int)\n").is_empty());
    }
    #[test]
    fn regular_class_with_fn_ok() {
        assert!(c("class Foo(val x: Int) { fun f() = x }\n").is_empty());
    }
}
