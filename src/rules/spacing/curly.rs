//! standard:curly-spacing — ensures spaces around curly braces.
//!
//! Rules:
//! - `{` should be preceded by a single space (unless at line start or after `(`, `[`)
//! - `{` should be followed by correct spacing (newline for bodies, space for lambdas)
//! - `}` should not be preceded by a space unless on same line as content

use crate::rules::{Rule, Violation};
use tree_sitter::Node;

pub struct CurlySpacing;

impl Rule for CurlySpacing {
    fn id(&self) -> &'static str {
        "standard:curly-spacing"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk_and_check(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl CurlySpacing {
    fn walk_and_check(&self, node: Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        if node.kind() == "{" {
            self.check_open_brace(&node, bytes, violations);
        } else if node.kind() == "}" {
            self.check_close_brace(&node, bytes, violations);
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk_and_check(child, bytes, violations);
            }
        }
    }

    fn check_open_brace(&self, node: &Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let pos = node.start_position();
        let start_byte = node.start_byte();

        // Skip over newlines: if preceded by \n, it's already at line start (fine)
        if start_byte > 0 {
            let prev_char = bytes[start_byte - 1];
            // tree-sitter columns are byte offsets, so this derives the line start in O(1).
            let line_start = start_byte.saturating_sub(pos.column);
            let is_first_token_on_line = bytes[line_start..start_byte]
                .iter()
                .all(|byte| byte.is_ascii_whitespace());
            if is_first_token_on_line {
                // Allman style: a `{` on its own line. Allowed for lambda
                // literals (a wrapped trailing lambda or a lambda argument
                // starts on its own line) and for the `when` block brace
                // (ktlint 1.8 exempts it), reported for block braces —
                // function body, class body, control flow — which must stay
                // on the same line as the declaration (issue #178).
                let parent_is_exempt = node
                    .parent()
                    .is_some_and(|p| matches!(p.kind(), "lambda_literal" | "when_expression"));
                if !parent_is_exempt {
                    violations.push(Violation {
                        file: String::new(),
                        line: pos.row + 1,
                        col: pos.column + 1,
                        rule_id: self.id().to_string(),
                        message: "Unexpected newline before \"{\"".to_string(),
                        auto_fixable: true,
                    });
                }
            } else if matches!(prev_char, b'(' | b'[') {
                // OK — lambda literals can directly follow an opening delimiter.
            } else if prev_char == b'@' {
                // OK — labeled lambda (`label@{ ... }`) has no space.
            } else if prev_char != b' ' {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 1,
                    rule_id: self.id().to_string(),
                    message: "Missing spacing before \"{\"".to_string(),
                    auto_fixable: true,
                });
            } else if start_byte >= 2 && bytes[start_byte - 2] == b' ' {
                // Double space before {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 1,
                    rule_id: self.id().to_string(),
                    message: "Too many spaces before \"{\"".to_string(),
                    auto_fixable: true,
                });
            }
        }
    }

    fn check_close_brace(&self, node: &Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let pos = node.start_position();
        let start_byte = node.start_byte();

        // A closing brace that starts its own line (only whitespace before it)
        // is a block closer — always fine regardless of AST quirks.
        let line_start = bytes[..start_byte]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        if bytes[line_start..start_byte]
            .iter()
            .all(|b| *b == b' ' || *b == b'\t')
        {
            return;
        }

        // } should be at the start of a line (possibly with indent) or preceded by a single space
        if start_byte > 0 {
            let prev_char = bytes[start_byte - 1];
            if prev_char == b'\n' {
                // OK — at line start
            } else if prev_char != b' '
                && prev_char != b'\t'
                && prev_char != b'{'
                && prev_char != b'}'
                && prev_char != b';'
            {
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 1,
                    rule_id: self.id().to_string(),
                    message: "Missing space before \"}\"".to_string(),
                    auto_fixable: false, // formatter does not fix closing braces
                });
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
        CurlySpacing.check(&tree, source)
    }

    #[test]
    fn valid_curly_spacing() {
        let source = "class Foo {\n    fun bar() {\n        return 1\n    }\n}\n";
        assert!(check(source).is_empty());
    }

    #[test]
    fn missing_space_before_open_brace() {
        let source = "class Foo{\n}\n";
        let v = check(source);
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:curly-spacing");
    }

    #[test]
    fn double_space_before_open_brace() {
        let source = "class Foo  {\n}\n";
        let v = check(source);
        assert!(!v.is_empty());
    }

    #[test]
    fn brace_after_paren_is_ok() {
        let source = "fun foo(): Int {\n    return 1\n}\n";
        let v = check(source);
        assert!(v.is_empty());
    }

    #[test]
    fn indented_lambda_argument_is_ok() {
        let source = "call(\n    { value() },\n)\n";
        assert!(check(source).is_empty());
    }

    // Issue #178: a block brace on its own line (Allman style) is reported.
    #[test]
    fn newline_before_function_body_brace_reported() {
        let source = "fun a(): Int\n{\n    return 1\n}\n";
        let v = check(source);
        assert!(
            v.iter()
                .any(|x| x.message.contains("Unexpected newline before \"{\"")),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn newline_before_control_flow_brace_reported() {
        let source = "fun b(x: Int): Int {\n    if (x > 0)\n    {\n        return 1\n    }\n    return 0\n}\n";
        let v = check(source);
        assert!(
            v.iter()
                .any(|x| x.message.contains("Unexpected newline before \"{\"")),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn newline_before_class_body_brace_reported() {
        let source = "class C\n{\n    val a = 1\n}\n";
        let v = check(source);
        assert!(
            v.iter()
                .any(|x| x.message.contains("Unexpected newline before \"{\"")),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn newline_before_try_and_finally_reported() {
        let source = "fun f() {\n    try\n    {\n        a()\n    }\n    finally\n    {\n        b()\n    }\n}\n";
        let v = check(source);
        assert!(
            v.iter()
                .any(|x| x.message.contains("Unexpected newline before \"{\"")),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    // ktlint 1.8 exempts the `when` block brace from the newline check.
    #[test]
    fn newline_before_when_block_brace_is_exempt() {
        let source = "fun f(x: Int) {\n    when (x)\n    {\n        1 -> println(1)\n    }\n}\n";
        let v = check(source);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn newline_before_when_entry_brace_reported() {
        // A when-entry block (`1 ->\n{`) is a control-structure body, not the
        // when block itself — ktlint reports it.
        let source = "fun f(x: Int) {\n    when (x) {\n        1 ->\n        {\n            a()\n        }\n    }\n}\n";
        let v = check(source);
        assert!(
            v.iter()
                .any(|x| x.message.contains("Unexpected newline before \"{\"")),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn lambda_on_own_line_still_ok() {
        // A wrapped trailing lambda starts on its own line — not reported.
        let source = "val x = list\n    .map\n    {\n        it * 2\n    }\n";
        let v = check(source);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    // ── Allman-brace regression battery (issue #178) ──
    // Every block context reports; lambda literals and the when block are
    // exempt; same-line braces are unaffected.

    fn newline_reports(source: &str) {
        let v = check(source);
        assert!(
            v.iter()
                .any(|x| x.message == "Unexpected newline before \"{\""),
            "expected newline-before-brace in:\n{source}\nviolations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn allman_for() {
        newline_reports("fun f() {\n    for (i in 0..1)\n    {\n        println(i)\n    }\n}\n");
    }

    #[test]
    fn allman_while() {
        newline_reports("fun f() {\n    while (true)\n    {\n        break\n    }\n}\n");
    }

    #[test]
    fn allman_do() {
        newline_reports("fun f() {\n    do\n    {\n        g()\n    } while (true)\n}\n");
    }

    #[test]
    fn allman_else() {
        newline_reports("fun f() {\n    if (x)\n    {\n        a()\n    }\n    else\n    {\n        b()\n    }\n}\n");
    }

    #[test]
    fn allman_catch() {
        newline_reports("fun f() {\n    try {\n        a()\n    } catch (e: Exception)\n    {\n        b()\n    }\n}\n");
    }

    #[test]
    fn allman_finally() {
        newline_reports(
            "fun f() {\n    try {\n        a()\n    } finally\n    {\n        b()\n    }\n}\n",
        );
    }

    #[test]
    fn allman_object_literal() {
        newline_reports("val o = object\n{\n    val a = 1\n}\n");
    }

    #[test]
    fn allman_init_block() {
        newline_reports("class C {\n    init\n    {\n        println()\n    }\n}\n");
    }

    #[test]
    fn allman_enum_body() {
        newline_reports("enum class E\n{\n    A\n}\n");
    }

    #[test]
    fn allman_companion_object() {
        newline_reports("class C {\n    companion object\n    {\n        val a = 1\n    }\n}\n");
    }

    #[test]
    fn allman_getter() {
        newline_reports("class C {\n    val a: Int\n        get()\n        {\n            return 1\n        }\n}\n");
    }

    #[test]
    fn allman_setter() {
        newline_reports("class C {\n    var a: Int = 0\n        set(value)\n        {\n            field = value\n        }\n}\n");
    }

    #[test]
    fn same_line_braces_untouched() {
        // The newline check must not fire for ordinary same-line braces.
        let src = "class Foo {\n    fun bar() {\n        if (x) { a() } else { b() }\n        return 1\n    }\n}\n";
        let v = check(src);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn missing_space_before_brace_still_reported() {
        let src = "fun f(){\n    return 1\n}\n";
        assert!(
            check(src)
                .iter()
                .any(|x| x.message == "Missing spacing before \"{\""),
            "missing-space must still fire"
        );
    }

    #[test]
    fn brace_after_comment_style_still_fine() {
        // Same-line braces are unaffected by the newline check.
        let source = "fun f() {\n    return 1\n}\n";
        assert!(check(source).is_empty());
    }
}
