use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

/// Checks that declarations (class, function, property) are preceded by a blank
/// line unless they're the first declaration in a file/class body.
/// JVM-compatible: checks both top-level and inside class bodies.
pub struct BlankLineBeforeDeclaration;

impl Rule for BlankLineBeforeDeclaration {
    fn id(&self) -> &'static str {
        "standard:blank-line-before-declaration"
    }

    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if is_declaration_block(node) {
                check_block(node, source, &mut violations);
            }
            let mut w = node.walk();
            let mut kids = Vec::new();
            for c in node.children(&mut w) {
                kids.push(c);
            }
            for c in kids.into_iter().rev() {
                stack.push(c);
            }
        }
        violations
    }
}

/// Declaration blocks where sibling declarations live: top level and the
/// bodies of class/object/interface/enum declarations (not function bodies —
/// local declarations are exempt, matching ktlint 1.8).
fn is_declaration_block(node: tree_sitter::Node) -> bool {
    matches!(
        node.kind(),
        "source_file"
            | "class_body"
            | "object_body"
            | "interface_body"
            | "enum_class_body"
            | "companion_object"
    )
}

fn is_declaration(node: tree_sitter::Node) -> bool {
    matches!(
        node.kind(),
        "class_declaration"
            | "object_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "annotation_declaration"
            | "function_declaration"
            | "property_declaration"
            | "type_alias"
            | "companion_object"
    )
}

fn is_property(node: tree_sitter::Node) -> bool {
    node.kind() == "property_declaration"
}

fn check_block(block: tree_sitter::Node, source: &str, out: &mut Vec<Violation>) {
    let mut w = block.walk();
    let children: Vec<tree_sitter::Node> = block.children(&mut w).collect();

    // Previous *code* sibling (comments/whitespace skipped) — used for the
    // first-code-sibling and consecutive-property exemptions.
    let mut prev_code: Option<tree_sitter::Node> = None;
    // Rows of comments directly above the current declaration, cleared by any
    // blank line or code between them and the declaration. ktlint reports at
    // the first leading-comment row (the PSI start offset includes it).
    let mut leading_comment_rows: Vec<usize> = Vec::new();

    for child in children {
        let kind = child.kind();
        // `{`/`}` (block delimiters) and whitespace are not code siblings.
        if kind == "{" || kind == "}" || kind.is_empty() {
            // Whitespace. A blank line breaks the leading-comment run.
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                if text.contains("\n\n") {
                    leading_comment_rows.clear();
                }
            }
            continue;
        }
        if kind.contains("comment") {
            leading_comment_rows.push(child.start_position().row);
            continue;
        }
        if is_declaration(child) {
            let first_code = prev_code.is_none();
            let consecutive_property = is_property(child) && prev_code.is_some_and(is_property);
            if !first_code && !consecutive_property {
                if let Some(prev) = prev_code {
                    check_pair(prev, &leading_comment_rows, child, source, out);
                }
            }
            leading_comment_rows.clear();
            prev_code = Some(child);
        } else {
            leading_comment_rows.clear();
            prev_code = Some(child);
        }
    }
}

/// Report when there is no blank line between the previous code and this
/// declaration's effective start (its leading comment, if any).
fn check_pair(
    prev: tree_sitter::Node,
    leading_comment_rows: &[usize],
    cur: tree_sitter::Node,
    source: &str,
    out: &mut Vec<Violation>,
) {
    let effective_row = leading_comment_rows
        .first()
        .copied()
        .unwrap_or(cur.start_position().row);
    // The effective start's line-start byte offset.
    let line_start = {
        let mut row = effective_row;
        let mut pos = 0usize;
        for line in source.split_inclusive('\n') {
            if row == 0 {
                break;
            }
            pos += line.len();
            row -= 1;
        }
        pos
    };
    // Anything between the previous code's end and the effective start must
    // not contain a blank line.
    let gap = &source[prev.end_byte().min(source.len())..line_start];
    if gap.contains("\n\n") {
        return;
    }
    // Column of the first non-whitespace character on the effective line
    // (ktlint reports the PSI start offset: col 1 for top-level, col 5 for an
    // indented member).
    let line_end = source[line_start..]
        .find('\n')
        .map_or(source.len(), |i| line_start + i);
    let content = &source[line_start..line_end];
    let col = content.len() - content.trim_start().len() + 1;
    out.push(Violation {
        file: String::new(),
        line: effective_row + 1,
        col,
        rule_id: "standard:blank-line-before-declaration".into(),
        message: "Expected a blank line for this declaration".into(),
        auto_fixable: true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        BlankLineBeforeDeclaration.check(&p.parse(src), src)
    }

    #[test]
    fn no_blank_between_declarations() {
        let v = check("fun a() {}\nfun b() {}");
        assert!(!v.is_empty());
    }

    #[test]
    fn blank_between_declarations() {
        let v = check("fun a() {}\n\nfun b() {}");
        assert!(v.is_empty());
    }

    #[test]
    fn first_declaration_no_blank() {
        let v = check("fun a() {}");
        assert!(v.is_empty());
    }

    #[test]
    fn adjacent_members_are_reported() {
        // ktlint 1.8 reports members without a blank line between them.
        let v = check("class Foo {\n    fun a() {}\n    fun b() {}\n}\n");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 3);
        assert_eq!(v[0].col, 5);
    }

    #[test]
    fn inside_class_body_with_blank() {
        let v = check("class Foo {\n    fun a() {}\n\n    fun b() {}\n}\n");
        assert!(v.is_empty());
    }

    #[test]
    fn first_in_class_body_no_blank_needed() {
        let v = check("class Foo {\n    fun a() {}\n}\n");
        assert!(v.is_empty());
    }
}
