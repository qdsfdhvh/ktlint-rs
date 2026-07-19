//! detekt:style:ClassOrdering — class members should follow a consistent order.
//! L1 rule using the resolver: properties before functions, constructors first.
use crate::rules::{Rule, Violation};
use tree_sitter::Node;

pub struct ClassOrdering;

impl Rule for ClassOrdering {
    fn id(&self) -> &'static str { "detekt:style:ClassOrdering" }
    fn auto_fixable(&self) -> bool { false }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(n) = stack.pop() {
            if n.kind() == "class_body" || n.kind() == "object_body" || n.kind() == "companion_object" {
                check_body_order(&n, bytes, &mut v);
            }
            for i in (0..n.child_count()).rev() {
                if let Some(c) = n.child(i) { stack.push(c); }
            }
        }
        v
    }
}

fn check_body_order(body: &Node, bytes: &[u8], v: &mut Vec<Violation>) {
    #[derive(PartialEq)]
    enum Section { Constructor, Property, Init, Function, None }
    let mut last = Section::None;
    for i in 0..body.child_count() {
        if let Some(child) = body.child(i) {
            let current = match child.kind() {
                "secondary_constructor" | "primary_constructor" => Section::Constructor,
                "property_declaration" => Section::Property,
                "init_block" => Section::Init,
                "function_declaration" => Section::Function,
                _ => continue,
            };
            // Constructors should come first, then properties, then init, then functions
            if last == Section::Function && current != Section::Function {
                let pos = child.start_position();
                v.push(Violation {
                    file: String::new(), line: pos.row + 1, col: pos.column + 1,
                    rule_id: "detekt:style:ClassOrdering".into(),
                    message: "Functions should be declared after properties and constructors".into(),
                    auto_fixable: false,
                });
            } else if last == Section::Init && (current == Section::Property || current == Section::Constructor) {
                let pos = child.start_position();
                v.push(Violation {
                    file: String::new(), line: pos.row + 1, col: pos.column + 1,
                    rule_id: "detekt:style:ClassOrdering".into(),
                    message: "Init blocks should come after property declarations".into(),
                    auto_fixable: false,
                });
            }
            last = current;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> { ClassOrdering.check(&KotlinParser::new().parse(s), s) }
    #[test]
    fn functions_before_properties_bad() {
        assert!(!c("class Foo { fun bar() {}\n    val x = 1\n}\n").is_empty());
    }
    #[test]
    fn constructor_then_properties_ok() {
        assert!(c("class Foo(val x: Int) { val y = 1\n    fun bar() {}\n}\n").is_empty());
    }
    #[test]
    fn empty_class_ok() {
        assert!(c("class Foo\n").is_empty());
    }
}
