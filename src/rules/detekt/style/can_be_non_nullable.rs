//! detekt:style:CanBeNonNullable — val x: String? = "hello" can be val x: String = "hello"
use crate::rules::{Rule, Violation};
use tree_sitter::Node;

pub struct CanBeNonNullable;

impl Rule for CanBeNonNullable {
    fn id(&self) -> &'static str {
        "detekt:style:CanBeNonNullable"
    }
    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        walk(tree.root_node(), bytes, &mut v);
        v
    }
}

fn walk(n: Node, bytes: &[u8], v: &mut Vec<Violation>) {
    if n.kind() == "property_declaration" {
        check_property(&n, bytes, v);
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            walk(c, bytes, v);
        }
    }
}

fn check_property(n: &Node, bytes: &[u8], v: &mut Vec<Violation>) {
    let text = n.utf8_text(bytes).unwrap_or("");

    // Must be a val (not var) with nullable type and an initializer
    if !text.contains("val ") {
        return;
    }
    if !text.contains("?") || !text.contains(" = ") {
        return;
    }
    // Skip lateinit, delegates, getters
    if text.contains("lateinit") || text.contains(" by ") {
        return;
    }

    // Check: type has `?` but initializer is non-null literal
    // Pattern: val name: Type? = <non-null-literal>
    // Simple heuristic: after `= ` there's no `?` and no `null`
    if let Some(init_part) = text.rsplit(" = ").next() {
        let init = init_part.trim();
        // Skip if init contains null, ?, or is a function call returning nullable
        if init == "null" || init.contains('?') || init.contains("get(") {
            return;
        }

        // Check that the type declaration has `?`
        let type_part = text.split(" = ").next().unwrap_or("");
        if type_part.contains(":") && type_part.contains("?") {
            let pos = n.start_position();
            v.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: "detekt:style:CanBeNonNullable".into(),
                message: "Nullable type with non-null initializer — can be non-nullable".into(),
                auto_fixable: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        CanBeNonNullable.check(&KotlinParser::new().parse(s), s)
    }
    #[test]
    fn nullable_with_non_null_init_bad() {
        assert!(!c("val x: String? = \"hello\"\n").is_empty());
    }
    #[test]
    fn nullable_with_null_init_ok() {
        assert!(c("val x: String? = null\n").is_empty());
    }
    #[test]
    fn non_nullable_ok() {
        assert!(c("val x: String = \"hello\"\n").is_empty());
    }
}
