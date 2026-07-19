//! standard:annotation — annotations on separate lines.
//! JVM-compatible: checks annotation nodes in declaration contexts
//! + inconsistent layout across adjacent annotation groups.

use crate::rules::{Rule, Violation};
use std::collections::BTreeMap;

pub struct AnnotationSpacing;

impl Rule for AnnotationSpacing {
    fn id(&self) -> &'static str {
        "standard:annotation"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();

        // Collect all declaration annotations grouped by line
        let mut line_annotations: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
        walk(tree.root_node(), bytes, &mut |node| {
            if node.kind() == "annotation" && is_decl_annotation(&node) {
                let pos = node.start_position();
                let line = pos.row + 1;
                let col = pos.column + 1;
                line_annotations.entry(line).or_default().push((line, col));
                // Individual checks per annotation
                check_annotation(&node, bytes, &mut v);
            }
        });
        // JVM: check inconsistent layout across adjacent annotation groups
        check_annotation_layout(&line_annotations, &mut v);
        v
    }
}

fn walk(
    root: tree_sitter::Node,
    bytes: &[u8],
    visit: &mut dyn FnMut(tree_sitter::Node),
) {
    let mut stack: Vec<tree_sitter::Node> = vec![root];
    while let Some(node) = stack.pop() {
        visit(node);
        for i in (0..node.child_count()).rev() {
            if let Some(c) = node.child(i) { stack.push(c); }
        }
    }
}

fn is_decl_annotation(node: &tree_sitter::Node) -> bool {
    let mut cur = node.parent();
    while let Some(p) = cur {
        match p.kind() {
            "class_declaration" | "function_declaration" | "property_declaration"
            | "object_declaration" | "companion_object" | "enum_entry"
            | "primary_constructor" | "secondary_constructor" | "type_alias"
            | "modifiers" => return true,
            "import_header" => return false,
            "class_parameters" | "function_value_parameters" => return true,
            "user_type" | "nullable_type" | "type_arguments" | "type_projection"
            | "function_type" | "annotated_type" | "value_arguments" | "call_expression"
            | "when_entry" | "when_expression" | "binary_expression" | "lambda_literal"
            | "return_expression" | "function_body" | "class_body" | "statements" => return false,
            _ => {}
        }
        cur = p.parent();
    }
    true
}

fn check_annotation(node: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    let pos = node.start_position();
    let line_start = node.start_byte().saturating_sub(pos.column);
    let in_params = in_parameter_list(node);

    let mut prev_was_annotation = false;
    let mut prev_was_code = false;
    let mut i = line_start;
    while i < node.start_byte() {
        match bytes[i] {
            b' ' | b'\t' => {}
            b'@' => prev_was_annotation = true,
            b'\n' => break,
            _ => prev_was_code = true,
        }
        i += 1;
    }

    if prev_was_code && !in_params {
        violations.push(Violation {
            file: String::new(), line: pos.row + 1, col: pos.column + 1,
            rule_id: "standard:annotation".into(),
            message: "Expected newline before annotation".into(),
            auto_fixable: true,
        });
    }
}

fn in_parameter_list(node: &tree_sitter::Node) -> bool {
    let mut cur = node.parent();
    while let Some(p) = cur {
        match p.kind() {
            "class_parameters" | "function_value_parameters" | "value_parameter" => return true,
            "class_declaration" | "function_declaration" | "property_declaration"
            | "object_declaration" => return false,
            _ => {}
        }
        cur = p.parent();
    }
    false
}

/// JVM-compatible: check inconsistent annotation layout across adjacent lines.
/// Pattern: @Foo on line N, @Bar @Baz on line N+1.
fn check_annotation_layout(
    groups: &BTreeMap<usize, Vec<(usize, usize)>>,
    violations: &mut Vec<Violation>,
) {
    // Collect consecutive annotation line groups and their annotation counts
    let mut prev_line: Option<usize> = None;
    let mut prev_count: Option<usize> = None;

    for (&line, anno_list) in groups.iter() {
        let count = anno_list.len();
        if let (Some(pl), Some(pc)) = (prev_line, prev_count) {
            // Adjacent lines (gap of 1) with different annotation counts
            if line == pl + 1 && pc != count && pc >= 1 && count >= 1 {
                violations.push(Violation {
                    file: String::new(),
                    line,
                    col: 1,
                    rule_id: "standard:annotation".into(),
                    message: "Inconsistent annotation layout: all annotations should be on separate lines or all on the same line".into(),
                    auto_fixable: true,
                });
            }
        }
        prev_line = Some(line);
        prev_count = Some(count);
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
    #[test] fn code_before_annotation_bad() { assert!(!check("class Foo @Inject\n").is_empty()); }
    #[test] fn three_annotations_first_clean() { let v = check("@A @B @C\nclass Foo\n"); assert!(v.len() >= 3); }
    #[test] fn annotation_inside_when_ok() { assert!(check("val x = when { is Foo -> @Suppress(\"bar\") 1 }\n").is_empty()); }
    /// JVM-compatible: inconsistent layout
    #[test] fn mixed_layout_bad() { assert!(!check("@Foo\n@Bar @Baz\nfun foo() {}\n").is_empty()); }
    #[test] fn consistent_layout_ok() { assert!(check("@Foo\n@Bar\n@Baz\nfun foo() {}\n").is_empty()); }
}
