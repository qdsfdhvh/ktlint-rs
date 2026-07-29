//! `standard:spacing-between-declarations-with-annotations`.

use crate::rules::{Rule, Violation};

pub struct SpacingBetweenDeclarationsWithAnnotations;

impl Rule for SpacingBetweenDeclarationsWithAnnotations {
    fn id(&self) -> &'static str {
        "standard:spacing-between-declarations-with-annotations"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        walk(tree.root_node(), &mut |node| {
            if !is_declaration(node.kind()) || !has_annotation(node, source) {
                return;
            }
            let Some(previous) = previous_named_sibling(node) else {
                return;
            };
            if !is_declaration(previous.kind()) {
                return;
            }
            let gap = &source[previous.end_byte()..node.start_byte()];
            if gap.bytes().filter(|byte| *byte == b'\n').count() >= 2 {
                return;
            }
            let position = node.start_position();
            violations.push(Violation {
                file: String::new(),
                line: position.row + 1,
                col: position.column + 1,
                rule_id: self.id().into(),
                message: "Declarations and declarations with annotations should have an empty space between.".into(),
                auto_fixable: true,
            });
        });
        violations
    }
}

fn is_declaration(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration"
            | "function_declaration"
            | "object_declaration"
            | "property_declaration"
            | "type_alias"
            | "secondary_constructor"
            | "getter"
            | "setter"
    )
}

fn has_annotation(node: tree_sitter::Node, source: &str) -> bool {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if current.kind() == "annotation"
            && source.as_bytes().get(current.start_byte()) == Some(&b'@')
        {
            return true;
        }
        // Do not inspect nested declaration bodies.
        if current != node && is_declaration(current.kind()) {
            continue;
        }
        for index in (0..current.child_count()).rev() {
            if let Some(child) = current.child(index) {
                stack.push(child);
            }
        }
    }
    false
}

fn previous_named_sibling(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut previous = node.prev_named_sibling();
    while previous.is_some_and(|candidate| candidate.kind().contains("comment")) {
        previous = previous.and_then(|candidate| candidate.prev_named_sibling());
    }
    previous
}

fn walk(root: tree_sitter::Node, visit: &mut impl FnMut(tree_sitter::Node)) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        visit(node);
        for index in (0..node.child_count()).rev() {
            if let Some(child) = node.child(index) {
                stack.push(child);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(source);
        SpacingBetweenDeclarationsWithAnnotations.check(&tree, source)
    }

    #[test]
    fn accepts_blank_line_before_annotated_declaration() {
        assert!(
            check("fun first() = Unit\n\n@Deprecated(\"x\")\nfun second() = Unit\n").is_empty()
        );
    }

    #[test]
    fn reports_annotation_without_separating_blank_line() {
        let violations = check("fun first() = Unit\n@Deprecated(\"x\")\nfun second() = Unit\n");
        assert_eq!(violations.len(), 1);
        assert_eq!((violations[0].line, violations[0].col), (2, 1));
        assert_eq!(
            violations[0].message,
            "Declarations and declarations with annotations should have an empty space between."
        );
        assert!(violations[0].auto_fixable);
    }
}
