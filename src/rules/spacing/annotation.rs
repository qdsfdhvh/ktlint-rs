//! standard:annotation-spacing — each annotation on its own line.

use crate::rules::{Rule, Violation};

pub struct AnnotationSpacing;

impl Rule for AnnotationSpacing {
    fn id(&self) -> &'static str {
        "standard:annotation-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk(tree.root_node(), bytes, source, &mut violations);
        violations
    }
}

impl AnnotationSpacing {
    fn walk(&self, node: tree_sitter::Node, bytes: &[u8], source: &str, violations: &mut Vec<Violation>) {
        // Find annotation nodes — they start with "@" in the annotations list
        if node.kind() == "annotation" {
            self.check_annotation(&node, bytes, source, violations);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, bytes, source, violations);
            }
        }
    }

    fn check_annotation(&self, node: &tree_sitter::Node, bytes: &[u8], _source: &str, violations: &mut Vec<Violation>) {
        let pos = node.start_position();
        let start_byte = node.start_byte();

        // Check if annotation is preceded by another annotation on the same line
        if start_byte > 0 && bytes[start_byte - 1] != b'\n' {
            // Count backwards to see if we're after another annotation
            let mut back = start_byte - 1;
            while back > 0 {
                if bytes[back] == b'\n' {
                    break;
                }
                if bytes[back] == b'@' {
                    violations.push(Violation {
                        file: String::new(),
                        line: pos.row + 1,
                        col: pos.column + 1,
                        rule_id: self.id().to_string(),
                        message: "Annotation should be on a separate line".to_string(),
                        auto_fixable: true,
                    });
                    break;
                }
                back -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        AnnotationSpacing.check(&tree, source)
    }

    #[test]
    fn single_annotation_ok() {
        let v = check("@Deprecated\nclass Foo\n");
        assert!(v.is_empty());
    }

    #[test]
    fn multiple_annotations_separate_lines_ok() {
        let v = check("@Deprecated\n@Suppress\nclass Foo\n");
        assert!(v.is_empty());
    }
}
