//! standard:annotation — annotations on separate lines.
//! Only flags: multiple annotations on same line, code after annotation block.
use crate::rules::{Rule, Violation};

pub struct AnnotationSpacing;

impl Rule for AnnotationSpacing {
    fn id(&self) -> &'static str { "standard:annotation" }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk(tree.root_node(), source.as_bytes(), &mut v);
        v
    }
}

fn walk(node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    if node.kind() == "modifiers" {
        let anns: Vec<tree_sitter::Node> =
            (0..node.child_count()).filter_map(|i| node.child(i)).filter(|c| c.kind() == "annotation").collect();

        for idx in 0..anns.len() {
            let ann = &anns[idx];
            let pos = ann.start_position();

            // Consecutive annotations on same line → flag
            if idx > 0 {
                let prev = &anns[idx - 1];
                if prev.end_position().row == pos.row {
                    violations.push(Violation {
                        file: String::new(), line: pos.row + 1, col: pos.column + 1,
                        rule_id: "standard:annotation".into(),
                        message: "Expected newline before annotation".into(),
                        auto_fixable: true,
                    });
                }
            }

            // Code after last annotation on same line → flag (only for multi-annotation blocks)
            if anns.len() > 1 && idx == anns.len() - 1 {
                let mut a = ann.end_byte();
                while a < bytes.len() {
                    if bytes[a] == b'\n' { break; }
                    if bytes[a] != b' ' && bytes[a] != b'\t' && bytes[a] != b'@' {
                        violations.push(Violation {
                            file: String::new(), line: pos.row + 1, col: pos.column + 1,
                            rule_id: "standard:annotation".into(),
                            message: "Expected newline after last annotation".into(),
                            auto_fixable: true,
                        });
                        break;
                    }
                    a += 1;
                }
            }
        }
    }

    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) { walk(c, bytes, violations); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> {
        AnnotationSpacing.check(&KotlinParser::new().parse(s), s)
    }

    #[test] fn single_annotation_newline_ok() { assert!(check("@Deprecated\nclass Foo\n").is_empty()); }
    #[test] fn single_annotation_same_line_ok() { assert!(check("@Deprecated class Foo\n").is_empty()); }
    #[test] fn two_annotations_separate_ok() { assert!(check("@A\n@B\nclass Foo\n").is_empty()); }
    #[test] fn two_annotations_same_line_bad() { assert!(!check("@A @B\nclass Foo\n").is_empty()); }
}
