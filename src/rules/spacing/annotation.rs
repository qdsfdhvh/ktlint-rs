//! standard:annotation — annotations on separate lines.
//! JVM-compatible: checks annotation nodes in declaration contexts
//! + inconsistent layout across adjacent annotation groups.

use crate::rules::{Rule, Violation};

pub struct AnnotationSpacing;

impl Rule for AnnotationSpacing {
    fn id(&self) -> &'static str {
        "standard:annotation"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();

        // Check syntax annotations in declaration contexts. The grammar also uses an
        // `annotation` node for the `annotation class` modifier, so require a leading `@`.
        walk(tree.root_node(), bytes, &mut |node| {
            if node.kind() == "annotation"
                && bytes.get(node.start_byte()) == Some(&b'@')
                && is_decl_annotation(&node)
            {
                check_annotation(&node, bytes, &mut v);
            }
        });
        // Issue #204: two annotations sharing a line (`@A("x") @B`) —
        // ktlint reports "Expected newline before annotation" at the second.
        walk(tree.root_node(), bytes, &mut |node| {
            if node.kind() == "modifiers" {
                let mut w = node.walk();
                let annotations: Vec<tree_sitter::Node> = node
                    .children(&mut w)
                    .filter(|c| c.kind() == "annotation")
                    .collect();
                // ktlint reports a second annotation sharing the line only
                // when the first one carries arguments (`@A("x") @B`); bare
                // `@A @B` is allowed.
                for pair in annotations.windows(2) {
                    let first_has_args = pair[0]
                        .utf8_text(bytes)
                        .is_ok_and(|t| t.find('(').is_some_and(|i| i > 1));
                    if first_has_args
                        && pair[0].start_position().row == pair[1].start_position().row
                    {
                        let pos = pair[1].start_position();
                        v.push(Violation {
                            file: String::new(),
                            line: pos.row + 1,
                            // Oracle reports one column before the second @
                            // (the whitespace between the two annotations).
                            col: pos.column,
                            rule_id: self.id().into(),
                            message: "Expected newline before annotation".into(),
                            auto_fixable: true,
                        });
                    }
                }
            }
        });
        check_same_line_annotation_groups(source, &mut v);
        v
    }
}

fn walk(root: tree_sitter::Node, _bytes: &[u8], visit: &mut dyn FnMut(tree_sitter::Node)) {
    let mut stack: Vec<tree_sitter::Node> = vec![root];
    while let Some(node) = stack.pop() {
        visit(node);
        for i in (0..node.child_count()).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
}

fn is_decl_annotation(node: &tree_sitter::Node) -> bool {
    // JVM-compatible: check all annotations except in imports.
    let mut cur = node.parent();
    while let Some(p) = cur {
        match p.kind() {
            "import_header" => return false,
            kind if kind.contains("string") || kind.contains("comment") => return false,
            // Reached a declaration context — stop walking, include it
            "class_declaration"
            | "function_declaration"
            | "property_declaration"
            | "object_declaration"
            | "companion_object"
            | "enum_entry"
            | "primary_constructor"
            | "secondary_constructor"
            | "type_alias"
            | "modifiers"
            | "class_parameters"
            | "function_value_parameters" => return true,
            // Type references and everything else: continue walking up
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
    let before = &bytes[line_start..node.start_byte()];
    let is_inline_type_annotation = before
        .iter()
        .rev()
        .find(|byte| !byte.is_ascii_whitespace())
        .is_some_and(|byte| matches!(*byte, b':' | b'=' | b'(' | b',' | b'<'));
    let line_end = bytes[node.end_byte()..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |offset| node.end_byte() + offset);
    let annotates_constructor = bytes[node.end_byte()..line_end]
        .windows("constructor".len())
        .any(|window| window == b"constructor");
    let indented_annotation_group = pos.column > 0
        && before
            .iter()
            .copied()
            .find(|byte| !byte.is_ascii_whitespace())
            == Some(b'@');

    let mut prev_was_code = false;
    let mut i = line_start;
    while i < node.start_byte() {
        match bytes[i] {
            b' ' | b'\t' => {}
            b'@' => {}
            b'\n' => break,
            _ => prev_was_code = true,
        }
        i += 1;
    }

    if prev_was_code && !in_params && !is_inline_type_annotation && !indented_annotation_group {
        violations.push(Violation {
            file: String::new(),
            line: pos.row + 1,
            col: pos.column + 1,
            rule_id: "standard:annotation".into(),
            message: "Expected newline before annotation".into(),
            auto_fixable: true,
        });
    }
}

/// ktlint 1.8: two or more annotations on one line are fine only when the
/// annotation group is on its own line. When the last annotation is followed
/// on the same line by a declaration, report "Expected newline after last
/// annotation" (issue #168). A single annotation (`@Composable fun ...`) is
/// fine. A group followed by a newline (`@Marker @Other\nval ...`) is fine.
fn check_same_line_annotation_groups(source: &str, violations: &mut Vec<Violation>) {
    for (line_index, line) in source.lines().enumerate() {
        let at_positions: Vec<usize> = line.match_indices('@').map(|(p, _)| p).collect();
        let Some(&last_at) = at_positions.last() else {
            continue;
        };
        // Skip the annotation name (`@Marker`) and any annotation arguments
        // in parens.
        let rest = &line[last_at + 1..];
        let name_len = rest
            .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .unwrap_or(rest.len());
        let after_name = rest[name_len..].trim_start();
        // The last annotation is followed on the same line by a declaration.
        // A primary `constructor` after the last annotation is always
        // separated (`@Inject constructor` -> `@Inject\nconstructor`, JVM
        // 1.8); other declaration keywords only when at least two annotations
        // share the line (`@A("x") @B val` — issue #168). A lone
        // `@Composable fun` / `@Inject val` stays put.
        let followed_by_decl = after_name.starts_with("constructor(")
            || (at_positions.len() >= 2
                && (after_name.starts_with("val ")
                    || after_name.starts_with("var ")
                    || after_name.starts_with("fun ")
                    || after_name.starts_with("class ")
                    || after_name.starts_with("object ")
                    || after_name.starts_with("interface ")
                    || after_name.starts_with("typealias ")));
        if followed_by_decl {
            violations.push(Violation {
                file: String::new(),
                line: line_index + 1,
                col: last_at + 1 + name_len + 1,
                rule_id: "standard:annotation".into(),
                message: "Expected newline after last annotation".into(),
                auto_fixable: true,
            });
        }
    }
}

fn in_parameter_list(node: &tree_sitter::Node) -> bool {
    let mut cur = node.parent();
    while let Some(p) = cur {
        match p.kind() {
            "class_parameters" | "function_value_parameters" | "value_parameter" => return true,
            "class_declaration"
            | "function_declaration"
            | "property_declaration"
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
    #[test]
    fn single_annotation_newline_ok() {
        assert!(check("@Deprecated\nclass Foo\n").is_empty());
    }
    #[test]
    fn single_annotation_same_line_ok() {
        assert!(check("@Deprecated class Foo\n").is_empty());
    }
    #[test]
    fn two_annotations_separate_ok() {
        assert!(check("@A\n@B\nclass Foo\n").is_empty());
    }
    #[test]
    fn two_annotations_same_line_followed_by_decl_bad() {
        // ktlint: a group followed on the same line by a declaration needs a
        // newline after the last annotation.
        assert!(!check("@A @B class Foo\n").is_empty());
        // A group on its own line is fine.
        assert!(check("@A @B\nclass Foo\n").is_empty());
    }
    #[test]
    fn code_before_annotation_bad() {
        assert!(!check("class Foo @Inject\n").is_empty());
    }
    #[test]
    fn three_annotations_first_clean() {
        let v = check("@A @B @C class Foo\n");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].message, "Expected newline after last annotation");
    }
    #[test]
    fn annotation_in_when_flagged() {
        assert!(!check("val x = when { is Foo -> @Suppress(\"bar\") 1 }\n").is_empty());
    }
    /// JVM-compatible: inconsistent layout
    #[test]
    fn mixed_layout_bad() {
        // `@Bar @Baz` on one line is fine (group on its own line); the
        // inconsistency comes from the declaration sharing the line.
        assert!(!check("@Foo\n@Bar @Baz fun foo() {}\n").is_empty());
    }
    #[test]
    fn consistent_layout_ok() {
        assert!(check("@Foo\n@Bar\n@Baz\nfun foo() {}\n").is_empty());
    }

    #[test]
    fn inline_type_annotations_and_at_in_strings_are_allowed() {
        assert!(check("typealias Content = @Composable (String) -> Unit\n").is_empty());
        assert!(check("val callback: @Composable () -> Unit\n").is_empty());
        assert!(check("val email = \"reader@@example.com\"\n").is_empty());
    }
}
