//! `standard:annotation-spacing` — annotations immediately precede their construct.

use crate::rules::{Rule, Violation};

pub struct AnnotationConstructSpacing;

impl Rule for AnnotationConstructSpacing {
    fn id(&self) -> &'static str {
        "standard:annotation-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        walk(tree.root_node(), &mut |node| {
            if node.kind() != "modifiers" {
                return;
            }
            let annotations = annotation_children(node, source);
            let Some(last) = annotations.last() else {
                return;
            };
            let Some(next) = node.next_sibling() else {
                return;
            };
            let span_end = next.start_byte();
            if span_end <= node.start_byte() || span_end > source.len() {
                return;
            }
            let span = &source[node.start_byte()..span_end];
            if has_blank_line(span) || has_intervening_comment(last.end_byte(), span_end, source) {
                let end = last.end_position();
                violations.push(Violation {
                    file: String::new(),
                    line: end.row + 1,
                    col: end.column + 1,
                    rule_id: self.id().into(),
                    message: "Annotations should occur immediately before the annotated construct"
                        .into(),
                    auto_fixable: true,
                });
            }
        });
        violations
    }
}

fn annotation_children<'tree>(
    modifiers: tree_sitter::Node<'tree>,
    source: &str,
) -> Vec<tree_sitter::Node<'tree>> {
    let bytes = source.as_bytes();
    let mut result = Vec::new();
    let mut stack = vec![modifiers];
    while let Some(node) = stack.pop() {
        if node.kind() == "annotation" && bytes.get(node.start_byte()) == Some(&b'@') {
            result.push(node);
            continue;
        }
        for index in (0..node.child_count()).rev() {
            if let Some(child) = node.child(index) {
                stack.push(child);
            }
        }
    }
    result.sort_by_key(tree_sitter::Node::start_byte);
    result
}

fn has_blank_line(text: &str) -> bool {
    let mut newlines = 0usize;
    for byte in text.bytes() {
        if byte == b'\n' {
            newlines += 1;
            if newlines >= 2 {
                return true;
            }
        } else if !byte.is_ascii_whitespace() && byte != b'@' {
            newlines = 0;
        }
    }
    false
}

fn has_intervening_comment(start: usize, end: usize, source: &str) -> bool {
    let between = &source[start..end];
    between.contains("//") || between.contains("/*")
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
        AnnotationConstructSpacing.check(&tree, source)
    }

    #[test]
    fn annotation_immediately_before_construct_is_valid() {
        assert!(check("@JvmField\nval value = 1\n").is_empty());
    }

    #[test]
    fn blank_line_after_annotation_matches_ktlint_position_and_message() {
        let violations = check("@JvmField\n\nval value = 1\n");
        assert_eq!(violations.len(), 1);
        assert_eq!((violations[0].line, violations[0].col), (1, 10));
        assert_eq!(
            violations[0].message,
            "Annotations should occur immediately before the annotated construct"
        );
        assert!(violations[0].auto_fixable);
    }

    #[test]
    fn multiple_annotations_report_at_last_annotation() {
        let violations = check("@JvmField\n@JvmStatic\n\nfun annotated() = Unit\n");
        assert_eq!(violations.len(), 1);
        assert_eq!((violations[0].line, violations[0].col), (2, 11));
    }
}
