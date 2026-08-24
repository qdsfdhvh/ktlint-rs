//! standard:paren-spacing — no space after `(`, no space before `)`.

use crate::rules::{Rule, Violation};

pub struct ParenSpacing;

impl Rule for ParenSpacing {
    fn id(&self) -> &'static str {
        "standard:paren-spacing"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk(tree.root_node(), bytes, source, &mut violations);
        violations
    }
}

impl ParenSpacing {
    fn walk(
        &self,
        node: tree_sitter::Node,
        bytes: &[u8],
        source: &str,
        violations: &mut Vec<Violation>,
    ) {
        let kind = node.kind();
        if kind == "(" {
            self.check_open_paren(&node, bytes, source, violations);
        } else if kind == ")" {
            self.check_close_paren(&node, bytes, violations);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, bytes, source, violations);
            }
        }
    }

    fn check_open_paren(
        &self,
        node: &tree_sitter::Node,
        bytes: &[u8],
        source: &str,
        violations: &mut Vec<Violation>,
    ) {
        let end_byte = node.end_byte();
        let start_byte = node.start_byte();
        let pos = node.start_position();

        let empty_split_list = end_byte < bytes.len()
            && bytes[end_byte] == b'\n'
            && node.parent().is_some_and(|p| {
                // An *empty* argument list split across lines (`foo(\n    )`)
                // is a spacing error; a multiline list with arguments is not
                // (ktlint 1.8, issue #170).
                matches!(
                    p.kind(),
                    "value_arguments" | "function_value_parameters" | "lambda_parameters"
                ) && p.child_count() <= 2
            });
        if (end_byte < bytes.len() && bytes[end_byte] == b' ') || empty_split_list {
            violations.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 2,
                rule_id: self.id().to_string(),
                message: "Unexpected spacing after \"(\"".to_string(),
                auto_fixable: true,
            });
        }
        // Issue #203: `foo (x)` — a space *before* the `(` of a call or a
        // parameter list is reported by ktlint's paren-spacing (keyword
        // constructs like `if (x)` keep their space and are not checked).
        //
        // Annotated function types (`@Composable (BoxScope.() -> Unit)?`)
        // must be exempt: tree-sitter-kotlin-sg mis-parses them as an
        // annotation constructor_invocation, so the `(` lands under a fake
        // `value_arguments` — the space before it separates the annotation
        // from the type, which is legal (oracle: clean, #260). Genuine
        // calls (`foo (x)`, `@Suppress ("x")`) keep the report. The
        // discriminator mirrors the oracle: a function type's `(` is never
        // a call's argument list — its content has a `->` arrow at depth 0.
        let paren_is_fn_type = node
            .parent()
            .is_some_and(|p| matches!(p.kind(), "value_arguments" | "function_value_parameters"))
            && (crate::rules::paren_content_has_top_level_arrow(node, source)
                // Multiline param lists mis-parse differently: the annotated
                // type's `(` becomes an EMPTY value_arguments (`@Composable (…`
                // with the content re-parented elsewhere), so the arrow scan
                // sees nothing. Discriminator: a space before `(` whose
                // preceding token is an `@Name` annotation — a genuine
                // annotation call (`@Suppress ("x")`) has non-empty content,
                // and `foo (x)` has no leading `@` (kataris corpus, #260).
                || annotation_leading_empty_paren(node, bytes)
                // A line-leading grouping paren (`    (current + turnId)
                // .takeLast(…)` — a wrapped expression's continuation row)
                // is mis-parsed as an empty value_arguments by
                // tree-sitter-kotlin-sg; the space before it is indentation,
                // legal (kataris corpus, oracle clean).
                || Self::grouping_paren_at_line_start(node, bytes));
        if node
            .parent()
            .is_some_and(|p| matches!(p.kind(), "value_arguments" | "function_value_parameters"))
            && !paren_is_fn_type
            && start_byte > 0
            && bytes[start_byte - 1] == b' '
        {
            violations.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: self.id().to_string(),
                message: "Unexpected spacing before \"(\"".to_string(),
                auto_fixable: true,
            });
        }
    }

    /// True when the `(` starts a grouping expression on its own row — the row
    /// up to `(` is pure indentation (a wrapped expression's continuation:
    /// `    (current + turnId)\n    .takeLast(…)`). tree-sitter-kotlin-sg
    /// mis-parses these as empty value_arguments, and the space before `(` is
    /// legal indentation (kataris corpus, oracle clean).
    fn grouping_paren_at_line_start(node: &tree_sitter::Node, bytes: &[u8]) -> bool {
        let start = node.start_byte();
        if start == 0 {
            return false;
        }
        // A real parameter list (`fun f\n    (x: Int)` mis-parse) is a
        // function_value_parameters parent — that is a call/declaration
        // paren, not a grouping row.
        if node
            .parent()
            .is_some_and(|p| p.kind() == "function_value_parameters")
        {
            return false;
        }
        let line_start = bytes[..start]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        bytes[line_start..start]
            .iter()
            .all(|&b| b == b' ' || b == b'\t')
    }

    fn check_close_paren(
        &self,
        node: &tree_sitter::Node,
        bytes: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        let start_byte = node.start_byte();
        let pos = node.start_position();

        if start_byte > 1
            && bytes[start_byte - 1] == b' '
            && bytes[start_byte - 2] != b' '
            && bytes[start_byte - 2] != b'\n'
        {
            // Don't flag if the space is part of an aligned parameter list
            // (this is a simplification — real ktlint checks for alignment)
            violations.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column,
                rule_id: self.id().to_string(),
                message: "Unexpected spacing before \")\"".to_string(),
                auto_fixable: true,
            });
        }
    }
}

/// Issue #260 (kataris): annotated/nested function types in a MULTILINE
/// parameter list mis-parse so a type's `(` becomes an EMPTY `value_arguments`
/// (`(` + `)` with the content re-parented elsewhere). Discriminator: a space
/// before `(` whose preceding token is an `@Name` annotation or a `name:`
/// typed-parameter colon — both legal `(` positions in a type (the `()` of
/// `@Composable (…` and of `onSuccess: () -> Unit`). A genuine annotation
/// call (`@Suppress ("x")`) and a real call (`foo (x)`) have non-empty
/// content; an empty annotation call (`@Suppress()`) has no space before `(`.
fn annotation_leading_empty_paren(node: &tree_sitter::Node, bytes: &[u8]) -> bool {
    let start_byte = node.start_byte();
    if start_byte == 0 || bytes[start_byte - 1] != b' ' {
        return false;
    }
    let in_param_list = node
        .parent()
        .is_some_and(|p| matches!(p.kind(), "value_arguments" | "function_value_parameters"));
    if !in_param_list {
        return false;
    }
    // Walk back over the whitespace run, then expect `@` + identifier chars
    // (annotation-then-type: `@Composable (…`) or identifier chars + `:`
    // (typed parameter: `onOpenMedia: (url: String, …) -> Unit`) — both are
    // function-TYPE parens where the space is legal.
    let mut j = start_byte - 1;
    while j > 0 && (bytes[j - 1] == b' ' || bytes[j - 1] == b'\t') {
        j -= 1;
    }
    let mut k = j;
    while k > 0 && (bytes[k - 1].is_ascii_alphanumeric() || bytes[k - 1] == b'_') {
        k -= 1;
    }
    let leading = if k > 0 && bytes[k - 1] == b'@' {
        Some(b'@')
    } else if k > 0 && bytes[k - 1] == b':' {
        Some(b':')
    } else {
        None
    };
    let Some(leading) = leading else { return false };
    // A genuine annotation call (`@Suppress ("x")`) has non-empty content
    // and must keep the report (oracle reports 3:10 for `@Suppress ("x")`)
    // — the `@` fallback only applies to an EMPTY misparsed list or a
    // type-starting content (`@Composable (StoryDetailSectionState) -> Unit`
    // keeps its parameter type in the list, starting with an uppercase
    // identifier, `(` or a nested `@`). A typed-parameter colon
    // (`name: (Type)`) is exempt regardless of content.
    if leading == b'@' {
        let empty_list = node.parent().is_some_and(|p| p.child_count() <= 2);
        if !empty_list {
            // Non-empty content: exempt only when it is function-type
            // shaped — a typed parameter (`draft: StoryPreviewDraft` — single
            // colon + space, NOT `Foo::class`), a `->` arrow, or a function
            // type whose return arrow follows the list (`(Type) -> Unit`). A
            // genuine annotation call (`@Suppress ("x")`, `@Suppress (x)`,
            // `@Suppress (Foo::class)`, `@Suppress (Foo)`) has none.
            let content_end = node.parent().map(|p| p.end_byte()).unwrap_or(start_byte);
            let content = std::str::from_utf8(
                &bytes[start_byte.saturating_add(1)..content_end.min(bytes.len())],
            )
            .unwrap_or("");
            let param_colon = content.contains(": ") && !content.contains("::");
            let arrow_in_content = content.contains("->");
            // A return arrow right after the misparsed list (`(Type) -> Unit`)
            // identifies a function type even when the list kept a bare
            // type name (`StoryDetailSectionState`).
            let after_end = content_end.min(bytes.len());
            let arrow_after = (0..16).any(|d| {
                let p = after_end.saturating_add(d);
                p + 1 < bytes.len() && bytes[p] == b'-' && bytes[p + 1] == b'>'
            });
            let type_like =
                !content.contains("::") && (param_colon || arrow_in_content || arrow_after);
            if !type_like {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        ParenSpacing.check(&tree, source)
    }

    #[test]
    fn valid_paren_spacing() {
        assert!(check("fun foo(a: Int)\n").is_empty());
    }

    #[test]
    fn space_after_open_paren() {
        let v = check("fun foo( a: Int)\n");
        assert!(!v.is_empty());
        assert!(v.iter().any(|x| x.message.contains("after")));
    }

    #[test]
    fn space_before_close_paren() {
        let v = check("fun foo(a: Int )\n");
        assert!(!v.is_empty());
        assert!(v.iter().any(|x| x.message.contains("before")));
    }

    #[test]
    fn empty_parens_ok() {
        assert!(check("fun foo()\n").is_empty());
    }
}

#[cfg(test)]
mod paren_annotated_call_negative_tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(src);
        ParenSpacing.check(&tree, src)
    }

    // Issue #260 negative: `@Suppress ("x")` is a genuine annotation call —
    // the space before `(` must still report (oracle 1.8: 3:10).
    #[test]
    fn suppress_call_with_space_reports() {
        let src = "package com.example\n\n@Suppress (\"unused\")\nfun b() {}\n";
        assert!(
            check(src)
                .iter()
                .any(|x| x.message.contains("before \"(\"")),
            "@Suppress (\"x\") must report paren-spacing"
        );
    }
}

#[cfg(test)]
mod paren_annotation_negative_tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(src);
        ParenSpacing.check(&tree, src)
    }

    // Issue #260 negatives: every spaced annotation CALL reports; only
    // function-type parens are exempt (oracle 1.8 reports all four).
    #[test]
    fn spaced_annotation_calls_report() {
        for src in [
            "@Suppress (\"unused\")\nfun b() {}\n",
            "@Suppress (x)\nfun b() {}\n",
            "@Suppress (Foo::class)\nfun b() {}\n",
            "@Suppress (Foo)\nfun b() {}\n",
        ] {
            let full = format!("package com.example\n\n{src}");
            assert!(
                check(&full)
                    .iter()
                    .any(|x| x.message.contains("before \"(\"")),
                "must report: {src:?}"
            );
        }
    }

    // The line-leading grouping exemption must not hide a real parameter
    // list paren (`fun f\n    (x: Int)` shape).
    #[test]
    fn line_leading_param_list_still_reports() {
        let src = "package com.example\n\nfun f\n    (x: Int) {\n    use(x)\n}\n";
        assert!(
            check(src)
                .iter()
                .any(|x| x.message.contains("before \"(\"")),
            "misparsed param list must report"
        );
    }
}
