//! standard:wrapping — general wrapping rule for when/if/for/while expressions.
//! Enforces that in multiline expressions, continuation elements are on new lines.
use crate::rules::{Rule, Violation};

pub struct GeneralWrapping;

impl Rule for GeneralWrapping {
    fn id(&self) -> &'static str {
        "standard:wrapping"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        // Issue #204: a function/class body's `{` must be followed by a
        // newline, and a multiline call/parameter list's `(` must not share
        // a line with its first argument.
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "{" {
                let block = node.parent().is_some_and(|p| match p.kind() {
                    "class_body" => true,
                    // A real function body belongs to a `fun` declaration; a
                    // lambda mis-parsed as function_body (the annotated
                    // function TYPE `((IntOffset) -> Unit)` corrupts the
                    // parse — kataris StoryCommentRow `remember(comment.id) {
                    // … }`) must not be newline-checked (oracle clean).
                    // A real function body belongs to a `fun` declaration;
                    // a lambda mis-parsed as function_body (the annotated
                    // function TYPE corrupts the parse — kataris StoryCommentRow
                    // `remember(comment.id) { … }` becomes a fake
                    // function_declaration with no `fun` keyword) must not be
                    // newline-checked (oracle clean).
                    "function_body" => p.parent().is_some_and(|g| {
                        if g.kind() != "function_declaration"
                            || !source[g.start_byte()..].starts_with("fun")
                        {
                            return false;
                        }
                        // The parameter list must end with `)` right before
                        // the body — an annotated function TYPE corrupts the
                        // parse so the fake params swallow the body content
                        // and never close (`… remember(comment.id)`), and the
                        // lambda `{` becomes the function body (kataris
                        // StoryCommentRow). A real `fun f(...) {` has a `)`.
                        let params_ok = g
                            .children(&mut g.walk())
                            .find(|c| c.kind() == "function_value_parameters")
                            .is_none_or(|params| {
                                source[params.end_byte()..].trim_start().starts_with(')')
                            });
                        params_ok
                    }),
                    _ => false,
                });
                if block {
                    report_after(node, bytes, source, '{', &mut violations);
                }
            } else if node.kind() == "(" || node.kind() == ")" {
                // A type-paren list (`@Composable (Type, () -> Unit) -> Unit`)
                // mis-parses as value_arguments whose SPAN is multiline even
                // though the type is single-line — the whole list must not be
                // wrapping-checked.
                let type_paren_list = node.parent().is_some_and(|p| {
                    matches!(p.kind(), "value_arguments" | "function_value_parameters")
                        && p.child(0).is_some_and(|open| {
                            leading_type_token(&open, bytes) || type_context_paren(&open, bytes)
                        })
                });
                let multiline = node.parent().is_some_and(|p| {
                    matches!(p.kind(), "value_arguments" | "function_value_parameters")
                        && p.start_position().row != p.end_position().row
                });
                // An annotated function type (`@Composable (BoxScope.() -> Unit)?`)
                // is mis-parsed by tree-sitter-kotlin-sg as an annotation
                // constructor call: its `(` lands under a fake multiline
                // `value_arguments`. The paren is a *type* paren, not an
                // argument list, so the wrapping check must not fire (oracle:
                // clean, #260).
                let paren_is_fn_type = (node.kind() == "(" || node.kind() == ")")
                    && node.parent().is_some_and(|p| {
                        matches!(p.kind(), "value_arguments" | "function_value_parameters")
                    })
                    && (crate::rules::paren_content_has_top_level_arrow(&node, source)
                        // Nested function types mis-parse their content away
                        // (`onOpenCoverMedia: ((url: String, …) -> Unit)?`), so
                        // the arrow scan sees an empty list — fall back to the
                        // `@Name ` / `name: ` leading-token discriminator
                        // (kataris corpus, #260).
                        || leading_type_token(&node, bytes)
                        || type_context_paren(&node, bytes));
                if multiline && !paren_is_fn_type && !type_paren_list {
                    if node.kind() == "(" {
                        report_after(node, bytes, source, '(', &mut violations);
                    } else {
                        // `"b")` — a closing paren sharing the last
                        // argument's line (issue #204).
                        let start = node.start_byte();
                        // A mis-parsed zero-width `)` node (annotated
                        // function type inflates the list) may not sit on an
                        // actual `)` byte — ignore it.
                        if bytes.get(start) != Some(&b')') {
                            continue;
                        }
                        // A type paren's `)` (`Modifier) -> Unit` inside an
                        // annotated function type) is not a call list's
                        // closing paren.
                        if source[start + 1..].trim_start().starts_with("->") {
                            continue;
                        }
                        // A trailing lambda after `)` (`remember(comment.id) {
                        // … }` — mis-parsed value_arguments when an annotated
                        // function TYPE corrupts the tree) is a call, not a
                        // multiline argument list; `) {` is legal (oracle
                        // clean, kataris StoryCommentRow).
                        if source[start + 1..].trim_start().starts_with('{') {
                            continue;
                        }
                        if start > 0 && bytes[start - 1] != b'\n' {
                            let prev_nonws = bytes[..start]
                                .iter()
                                .rposition(|&b| b != b' ' && b != b'\t' && b != b'\n');
                            let last_arg = node.parent().is_some_and(|p| {
                                p.children(&mut p.walk()).any(|c| {
                                    matches!(c.kind(), "value_argument" | "parameter")
                                        && c.end_position().row == node.start_position().row
                                })
                            });
                            // `})` — the paren closes a list whose last
                            // content is a lambda (`withTransform({ … })`,
                            // Compose graphics) — ktlint stays silent.
                            let prev_is_brace =
                                prev_nonws.and_then(|i| bytes.get(i)) == Some(&b'}');
                            if last_arg && !prev_is_brace {
                                // oracle reports at the char before `)`
                                // (`beta: String)` -> 4:16).
                                let col = prev_nonws
                                    .map(|i| {
                                        let line_start = bytes[..i]
                                            .iter()
                                            .rposition(|&b| b == b'\n')
                                            .map_or(0, |j| j + 1);
                                        i - line_start + 1
                                    })
                                    .unwrap_or(node.start_position().column + 1);
                                violations.push(Violation {
                                    file: String::new(),
                                    line: node.start_position().row + 1,
                                    col,
                                    rule_id: self.id().into(),
                                    message: "Missing newline before \")\"".into(),
                                    auto_fixable: true,
                                });
                            }
                        }
                    }
                }
            }
            for i in (0..node.child_count()).rev() {
                if let Some(c) = node.child(i) {
                    stack.push(c);
                }
            }
        }
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            // Check `when` entries — `1, 2 ->` on same line is wrong for multiline when
            if trimmed.starts_with("when ") && i + 1 < lines.len() {
                let mut prev_line_was_entry = false;
                for t in lines[i + 1..]
                    .iter()
                    .take(lines.len().min(i + 50).saturating_sub(i + 1))
                    .map(|l| l.trim())
                {
                    if t == "}" || t == "})" || t == ")." {
                        break;
                    }
                    if t.contains("->") && !t.contains("\"") {
                        if prev_line_was_entry {
                            // Multiple entries on consecutive lines — check consistency
                        }
                        prev_line_was_entry = true;
                    } else if !t.is_empty() && !t.starts_with("//") {
                        prev_line_was_entry = false;
                    }
                }
            }

            // Check `if/else` chain consistency
            if (trimmed.starts_with("if (")
                || trimmed.starts_with("for (")
                || trimmed.starts_with("while ("))
                && trimmed.ends_with('{')
                && i + 1 < lines.len()
                && lines[i + 1].trim().is_empty()
            {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().to_string(),
                    message: "Unexpected blank line after if-condition".to_string(),
                    auto_fixable: true,
                });
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        GeneralWrapping.check(&p.parse(s), s)
    }
    #[test]
    fn if_no_blank() {
        assert!(check("if (x) {\n    doA()\n}\n").is_empty());
    }
    #[test]
    fn if_with_blank() {
        let v = check("if (x) {\n\n    doA()\n}\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:wrapping");
    }

    #[test]
    fn loop_with_blank() {
        assert!(!check("for (x in xs) {\n\n    use(x)\n}\n").is_empty());
        assert!(!check("while (ready) {\n\n    tick()\n}\n").is_empty());
    }
}

/// Report when a non-whitespace char follows the delimiter on the same line.
fn report_after(
    node: tree_sitter::Node,
    bytes: &[u8],
    source: &str,
    delim: char,
    violations: &mut Vec<Violation>,
) {
    let start = node.end_byte();
    let line_end = bytes[start..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |i| start + i);
    if let Some(off) = bytes[start..line_end]
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
    {
        let pos = start + off;
        // An empty block `{}` is fine on one line.
        if delim == '{' && bytes.get(pos) == Some(&b'}') {
            return;
        }
        // A lambda ANYWHERE on the `(` opener row
        // (`MessageAvatar(item, onClick = {\n … })`, `scaleClickable(onClick =
        // {`, `withTransform({\n … }`) exempts the list — ktlint 1.8 keeps
        // named-argument lambdas on the callee line, while a lambda-free
        // multiline list (`fun interaction(alpha: String,\n …)`) still
        // reports (oracle Interaction.kt).
        if delim == '('
            && bytes[start..line_end.min(bytes.len())]
                .iter()
                .any(|&b| b == b'{')
        {
            return;
        }
        // `({` — a paren list whose first content is a lambda (`withTransform({\n … })
        // — Compose graphics API) needs no newline after `(`: the lambda is the
        // only content, oracle stays silent (kataris corpus).
        if delim == '(' && bytes.get(pos) == Some(&b'{') {
            return;
        }
        // Oracle reports at the delimiter itself: `{ x` → the `{` column.
        let line = source[..start].bytes().filter(|&b| b == b'\n').count() + 1;
        let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
        violations.push(Violation {
            file: String::new(),
            line,
            col: start - line_start + 1,
            rule_id: "standard:wrapping".into(),
            message: format!("Missing newline after \"{delim}\""),
            auto_fixable: true,
        });
    }
}

/// True when the `(`/`)` is preceded (skipping the space run) by an `@Name`
/// annotation or a `name:` typed-parameter colon — the paren is a function
/// TYPE paren (`@Composable (…`, `onOpenMedia: (url: String, …) -> Unit`),
/// legal with a space before it. A real call (`foo (x)`, `@Suppress ("x")`)
/// has a bare identifier before the space (issue #260 / kataris).
fn leading_type_token(node: &tree_sitter::Node, bytes: &[u8]) -> bool {
    let start = node.start_byte();
    if start == 0 || bytes[start - 1] != b' ' {
        return false;
    }
    let mut j = start - 1;
    while j > 0 && (bytes[j - 1] == b' ' || bytes[j - 1] == b'\t') {
        j -= 1;
    }
    let mut k = j;
    while k > 0
        && (bytes[k - 1].is_ascii_alphanumeric() || bytes[k - 1] == b'_' || bytes[k - 1] == b'.')
    {
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
    // `@Name ` needs an EMPTY misparsed list or type-starting content (a
    // genuine `@Suppress ("x")` keeps the wrapping report); a
    // typed-parameter `name: ` colon is exempt regardless of content.
    if leading == b'@' {
        let empty_list = node.parent().is_some_and(|p| p.child_count() <= 2);
        if !empty_list {
            // Non-empty content: exempt only when function-type shaped
            // (`@Composable (draft: …)` — a `:` typed-param or `->` arrow).
            let content_end = node.parent().map(|p| p.end_byte()).unwrap_or(start);
            let content =
                std::str::from_utf8(&bytes[start.saturating_add(1)..content_end.min(bytes.len())])
                    .unwrap_or("");
            let type_like = content.contains(':')
                || content.contains("->")
                || content.trim_start().starts_with(char::is_uppercase);
            if !type_like {
                return false;
            }
        }
    }
    true
}

/// True when the character before the whitespace run before `(`/`)` is NOT
/// an identifier — the paren is a type/grouping paren, not a call argument
/// list (kataris `@Composable (Type?, () -> Unit) -> Unit`).
fn type_context_paren(node: &tree_sitter::Node, bytes: &[u8]) -> bool {
    let start = node.start_byte();
    if start == 0 || bytes[start - 1] != b' ' {
        return false;
    }
    let mut j = start - 1;
    while j > 0 && (bytes[j - 1] == b' ' || bytes[j - 1] == b'\t') {
        j -= 1;
    }
    j > 0 && !bytes[j - 1].is_ascii_alphanumeric() && bytes[j - 1] != b'_'
}

#[cfg(test)]
mod wrapping_type_paren_tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(src);
        GeneralWrapping.check(&tree, src)
    }

    // A parameter typed as an annotated function type
    // (`page: @Composable (CastUi, (CastUi) -> Unit) -> Unit,`) mis-parses
    // the closing paren row; `): DraftHandle {` on its own line is legal
    // (oracle clean, kataris StoryEditorCastBehaviorTest.kt:877).
    #[test]
    fn closing_paren_on_own_row_after_fn_type_param_ok() {
        let src = "package com.example\n\nprivate fun renderEditorPage(\n    character: CastUi,\n    page: @Composable (CastUi, (CastUi) -> Unit) -> Unit,\n): DraftHandle {\n    val handle = DraftHandle(character)\n    return handle\n}\n";
        assert!(check(src).is_empty());
    }

    // A named-argument lambda on the `(` row (`MessageAvatar(item, onClick =
    // {\n … })`) stays on the callee line (oracle clean, kataris MessageRow).
    #[test]
    fn named_arg_lambda_on_opener_row_ok() {
        let src = "package com.example\n\nfun f(item: String) {\n    MessageAvatar(item, onClick = {\n        onRead()\n        onMark()\n    })\n    use(item)\n}\n";
        assert!(check(src).is_empty());
    }

    // A lambda mis-parsed as a function body (annotated function TYPE
    // corrupts the tree — `remember(comment.id) { … }`) must not trigger
    // "Missing newline after {" (oracle clean, kataris StoryCommentRow).
    #[test]
    fn lambda_misparsed_as_function_body_ok() {
        let src = "package com.example\n\nfun f(\n    modifier: Modifier = Modifier,\n    content: @Composable ((IntOffset) -> Unit) -> Unit,\n) {\n    var actionMenuExpanded by remember(comment.id) { mutableStateOf(false) }\n    use(actionMenuExpanded)\n}\n";
        assert!(check(src).is_empty());
    }
}
