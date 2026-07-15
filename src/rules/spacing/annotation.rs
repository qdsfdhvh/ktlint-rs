//! standard:annotation — annotations on separate lines.
//! JVM-compatible: checks ALL annotation nodes in declaration contexts.

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

fn walk(root: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    let mut stack: Vec<tree_sitter::Node> = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "annotation" && is_decl_annotation(&node) {
            check_annotation(&node, bytes, violations);
        }
        for i in (0..node.child_count()).rev() {
            if let Some(c) = node.child(i) { stack.push(c); }
        }
    }
}

fn is_decl_annotation(node: &tree_sitter::Node) -> bool {
    let mut cur = node.parent();
    while let Some(p) = cur {
        match p.kind() {
            "class_declaration"
            | "function_declaration"
            | "property_declaration"
            | "object_declaration"
            | "companion_object"
            | "enum_entry"
            | "primary_constructor"
            | "secondary_constructor"
            | "type_alias"
            | "modifiers" => return true,
            "class_parameters" | "function_value_parameters" => return true,
            "user_type" | "nullable_type" | "type_arguments" | "type_projection"
            | "function_type" | "annotated_type"
            | "value_arguments" | "call_expression" | "when_entry" | "when_expression"
            | "binary_expression" | "lambda_literal" | "return_expression"
            | "function_body" | "class_body" | "statements" => return false,
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
            b'@' => { prev_was_annotation = true; }
            b'\n' => break,
            _ => { prev_was_code = true; }
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
        return;
    }
    if prev_was_annotation {
        violations.push(Violation {
            file: String::new(), line: pos.row + 1, col: pos.column + 1,
            rule_id: "standard:annotation".into(),
            message: "Multiple annotations should be placed on separate lines".into(),
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
    #[test] fn three_annotations_first_clean() {
        let v = check("@A @B @C\nclass Foo\n");
        assert_eq!(v.len(), 2);
    }
    #[test] fn annotation_inside_when_ok() { assert!(check("val x = when { is Foo -> @Suppress(\"bar\") 1 }\n").is_empty()); }
}
