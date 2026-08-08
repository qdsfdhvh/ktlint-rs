//! standard:spacing-between-declarations-with-comments
//!
//! ktlint 1.8: when a comment (KDoc or line comment) sits between two
//! sibling declarations in a declaration block (top level, class/object/
//! interface body) without a blank line on either side of it, the comment
//! must be separated from the declarations by an empty line. Verified
//! against the real ktlint 1.8 CLI:
//!
//! ```text
//! class A {}
//! // comment
//! class B {}          # violation at the comment line
//!
//! class A {}
//!                      # (blank line) — no violation
//! // comment
//! class B {}
//! ```
//!
//! Local statements inside function bodies are not checked. The rule only
//! fires for sibling declarations directly adjacent in the block — if any
//! non-comment, non-whitespace node sits between them, they are not a pair.

use crate::rules::{Rule, Violation};

pub struct SpacingBetweenDeclarationsWithComments;

impl Rule for SpacingBetweenDeclarationsWithComments {
    fn id(&self) -> &'static str {
        "standard:spacing-between-declarations-with-comments"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
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
/// bodies of class/object/interface/enum declarations.
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
            | "enum_entry"
    )
}

fn check_block(node: tree_sitter::Node, source: &str, out: &mut Vec<Violation>) {
    let mut w = node.walk();
    let children: Vec<tree_sitter::Node> = node.children(&mut w).collect();

    let mut prev_decl: Option<tree_sitter::Node> = None;
    // True while scanning between two declarations we have only seen
    // whitespace and comments — i.e. the declarations are direct siblings.
    let mut adjacent = true;
    for child in children {
        if is_declaration(child) {
            if adjacent {
                if let Some(prev) = prev_decl {
                    check_pair(prev, child, source, out);
                }
            }
            prev_decl = Some(child);
            adjacent = true;
        } else if !is_blank_or_comment(child) {
            // A non-comment, non-whitespace node (init block, accessor,
            // expression) sits between the declarations — not a direct pair.
            adjacent = false;
        }
    }
}

fn is_blank_or_comment(node: tree_sitter::Node) -> bool {
    let k = node.kind();
    k == "comment" || k.contains("comment") || k.is_empty()
}

fn check_pair(
    prev: tree_sitter::Node,
    cur: tree_sitter::Node,
    source: &str,
    out: &mut Vec<Violation>,
) {
    let gap = &source[prev.end_byte()..cur.start_byte()];
    // A blank line anywhere in the gap means the comment is properly
    // separated — no violation (ktlint requires a blank line on *both*
    // sides, so any blank line anywhere makes it compliant).
    if gap.contains("\n\n") {
        return;
    }
    // `*` (KDoc continuation or block-comment interior) counts too: a bare
    // `*` on a KDoc line is always inside a `/* ... */` span, so any `*`
    // in the gap implies a comment opened earlier in it.
    let comment_rel = gap
        .find("//")
        .or_else(|| gap.find("/*"))
        .or_else(|| gap.find("*"));
    if let Some(rel) = comment_rel {
        let abs = prev.end_byte() + rel;
        if std::env::var("SPACING_DEBUG").is_ok() {
            let ls = source[..abs].rfind('\n').map_or(0, |i| i + 1);
            eprintln!(
                "PAIR prev L{}..{} cur L{}..{} gap={:?} rel={rel} before={:?}",
                prev.start_position().row + 1,
                prev.end_position().row + 1,
                cur.start_position().row + 1,
                cur.end_position().row + 1,
                gap,
                &source[ls..abs]
            );
        }
        // Only a comment on its own line (leading whitespace then `//` or
        // `/*`) separates declarations — a trailing comment on the previous
        // declaration's line (`fun get(...): X // note`) does not.
        let line_start = source[..abs].rfind('\n').map_or(0, |i| i + 1);
        if !source[line_start..abs].trim().is_empty() {
            return;
        }
        let line = source[..abs].bytes().filter(|&b| b == b'\n').count() + 1;
        let col = abs - line_start + 1;
        out.push(Violation {
            file: String::new(),
            line,
            col,
            rule_id: "standard:spacing-between-declarations-with-comments".into(),
            message:
                "Declarations and declarations with comments should have an empty space between."
                    .into(),
            auto_fixable: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(src);
        SpacingBetweenDeclarationsWithComments.check(&tree, src)
    }

    #[test]
    fn comment_between_declarations_no_blank_line() {
        let src = "class A {}\n// comment\nclass B {}\n";
        let v = check(src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 2);
        assert_eq!(v[0].col, 1);
        assert_eq!(
            v[0].rule_id,
            "standard:spacing-between-declarations-with-comments"
        );
    }

    #[test]
    fn blank_line_on_either_side_is_fine() {
        // Blank before the comment.
        assert!(check("class A {}\n\n// comment\nclass B {}\n").is_empty());
        // Blank after the comment.
        assert!(check("class A {}\n// comment\n\nclass B {}\n").is_empty());
    }

    #[test]
    fn kdoc_between_declarations() {
        assert_eq!(check("class A {}\n/** doc */\nclass B {}\n").len(), 1);
        assert!(check("class A {}\n\n/** doc */\nclass B {}\n").is_empty());
    }

    #[test]
    fn trailing_comment_is_not_a_separator() {
        // `// ip stack` is a trailing comment on the declaration line.
        let src = "class A {\n    var host: String?\n    var family: Int? // ip stack\n    var noDelay: Boolean?\n}\n";
        assert!(check(src).is_empty());
    }

    #[test]
    fn local_statements_inside_functions_not_checked() {
        let src = "class A {\n    fun f() {\n        val x = 1\n        // comment\n        val y = 2\n    }\n}\n";
        assert!(check(src).is_empty());
    }

    #[test]
    fn blank_line_within_gap_anywhere_is_compliant() {
        let src = "class A {\n    val x = 1\n\n    // comment\n    val y = 2\n}\n";
        assert!(check(src).is_empty());
    }

    #[test]
    fn properties_in_class_body() {
        let src = "class A {\n    val x = 1\n    // comment\n    val y = 2\n}\n";
        let v = check(src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 3);
    }

    #[test]
    fn non_adjacent_declarations_not_a_pair() {
        // An init block between the declarations breaks adjacency.
        let src =
            "class A {\n    val x = 1\n    init { println(x) }\n    // comment\n    val y = 2\n}\n";
        assert!(check(src).is_empty());
    }
}
