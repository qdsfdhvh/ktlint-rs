//! Phase 3 final: type-argument-list, angle-brackets, function-signature, enum-wrapping, trailing-comma-*
use crate::rules::{Rule, Violation};

pub struct TypeArgumentListSpacing;
impl Rule for TypeArgumentListSpacing {
    fn id(&self) -> &'static str {
        "standard:type-argument-list-spacing"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if matches!(
                node.kind(),
                "type_arguments" | "type_parameters" | "type_projection"
            ) {
                for index in 0..node.child_count() {
                    let Some(child) = node.child(index) else {
                        continue;
                    };
                    let pos = child.start_position();
                    let offset = child.start_byte();
                    let line_start = bytes[..offset]
                        .iter()
                        .rposition(|&b| b == b'\n')
                        .map_or(0, |i| i + 1);
                    let only_indent = bytes[line_start..offset]
                        .iter()
                        .all(|&b| b == b' ' || b == b'\t');
                    let has_unexpected_space = match child.kind() {
                        "<" => child.end_byte() < bytes.len() && bytes[child.end_byte()] == b' ',
                        ">" => offset > 0 && bytes[offset - 1] == b' ' && !only_indent,
                        _ => false,
                    };
                    if has_unexpected_space {
                        violations.push(Violation {
                            file: String::new(),
                            line: pos.row + 1,
                            col: pos.column + 1,
                            rule_id: self.id().into(),
                            message: "No whitespace expected at this position".into(),
                            auto_fixable: true,
                        });
                    }
                }
            }
            for index in (0..node.child_count()).rev() {
                if let Some(child) = node.child(index) {
                    stack.push(child);
                }
            }
        }
        violations
    }
}

pub struct SpacingAroundAngleBrackets;
impl Rule for SpacingAroundAngleBrackets {
    fn id(&self) -> &'static str {
        "standard:spacing-around-angle-brackets"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        Self::walk(tree.root_node(), bytes, &mut v);
        v
    }
}
impl SpacingAroundAngleBrackets {
    fn walk(node: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
        let kind = node.kind();
        if (kind == "<" || kind == ">") && Self::in_generics_ctx(&node) {
            let pos = node.start_position();
            let s = node.start_byte();
            if kind == ">" && s > 0 && bytes[s - 1] == b' ' {
                // A `>` that starts its own line (only indentation before it,
                // e.g. a wrapped generic parameter list) is fine.
                let line_start = bytes[..s]
                    .iter()
                    .rposition(|&b| b == b'\n')
                    .map_or(0, |i| i + 1);
                let only_indent = bytes[line_start..s]
                    .iter()
                    .all(|&b| b == b' ' || b == b'\t');
                if !only_indent {
                    v.push(Violation {
                        file: String::new(),
                        line: pos.row + 1,
                        col: pos.column + 1,
                        rule_id: "standard:spacing-around-angle-brackets".into(),
                        message: "Unexpected spacing before \">\" in generics".into(),
                        auto_fixable: true,
                    });
                }
            }
            if kind == "<" {
                let e = node.end_byte();
                if e < bytes.len() && bytes[e] == b' ' {
                    v.push(Violation {
                        file: String::new(),
                        line: pos.row + 1,
                        col: pos.column + 1,
                        rule_id: "standard:spacing-around-angle-brackets".into(),
                        message: "Unexpected spacing after \"<\" in generics".into(),
                        auto_fixable: true,
                    });
                }
            }
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                Self::walk(c, bytes, v);
            }
        }
    }
    fn in_generics_ctx(node: &tree_sitter::Node) -> bool {
        node.parent().is_some_and(|p| {
            matches!(
                p.kind(),
                "type_arguments" | "type_parameters" | "type_projection"
            )
        })
    }
}

pub struct EnumWrapping;
impl Rule for EnumWrapping {
    fn id(&self) -> &'static str {
        "standard:enum-wrapping"
    }
    fn check(&self, tree: &tree_sitter::Tree, _source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "enum_class_body"
                && node.start_position().row != node.end_position().row
            {
                // Multiline enum: every entry after the first on a shared line
                // is reported (ktlint 1.8, issue #168).
                let mut w = node.walk();
                let entries: Vec<tree_sitter::Node> = node
                    .children(&mut w)
                    .filter(|c| c.kind() == "enum_entry")
                    .collect();
                for pair in entries.windows(2) {
                    if pair[0].end_position().row == pair[1].start_position().row {
                        let p = pair[1].start_position();
                        v.push(Violation {
                            file: String::new(),
                            line: p.row + 1,
                            col: p.column + 1,
                            rule_id: self.id().into(),
                            message: "Enum entry should start on a separate line".into(),
                            auto_fixable: true,
                        });
                    }
                }
            }
            let mut w2 = node.walk();
            let mut kids = Vec::new();
            for c in node.children(&mut w2) {
                kids.push(c);
            }
            for c in kids.into_iter().rev() {
                stack.push(c);
            }
        }
        v
    }
}

pub struct TrailingCommaOnDeclarationSite {
    /// Missing direction: a multiline list without a trailing comma is
    /// reported when the style demands it — ktlint_official/intellij_idea
    /// always, android_studio under `ij_kotlin_allow_trailing_comma=true`.
    require_trailing_comma: bool,
    /// Unnecessary direction (multiline lists): android_studio with
    /// trailing commas disabled reports them. Single-line lists report the
    /// comma as unnecessary under *every* style.
    forbid_trailing_comma: bool,
}

impl TrailingCommaOnDeclarationSite {
    pub fn new(allow_trailing_comma: bool, is_android_studio: bool) -> Self {
        Self {
            require_trailing_comma: allow_trailing_comma || !is_android_studio,
            forbid_trailing_comma: is_android_studio && !allow_trailing_comma,
        }
    }
}

impl Rule for TrailingCommaOnDeclarationSite {
    fn id(&self) -> &'static str {
        "standard:trailing-comma-on-declaration-site"
    }
    fn check(&self, tree: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let bytes = s.as_bytes();
        let mut v = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            let (elem_kinds, close_msg, lambda) = match node.kind() {
                "function_value_parameters" => (ELEM_PARAM, ")", false),
                // tree-sitter-kotlin flattens class constructor params into
                // primary_constructor (no class_parameters node)
                "primary_constructor" => (ELEM_CLASS_PARAM, ")", false),
                "lambda_parameters" => (ELEM_VAR_DECL, "->", true),
                "enum_class_body" => (ELEM_ENUM_ENTRY, "}", false),
                _ => (0, "", false),
            };
            if elem_kinds != 0 {
                let is_param_list = matches!(
                    node.kind(),
                    "function_value_parameters" | "primary_constructor"
                );
                // A function-type parameter (`storyContent: @Composable
                // (character: X, modifier: Y) -> Unit`) is mis-parsed by
                // tree-sitter-kotlin-sg into several top-level parameters
                // (the type's own `,` becomes a list separator) — the list's
                // trailing comma can't be measured reliably and oracle stays
                // silent (kataris corpus). Detect the `: @Name (` / `: (`
                // type-paren marker inside the list and skip it.
                let list_text = if is_param_list {
                    std::str::from_utf8(&bytes[node.start_byte()..node.end_byte()]).unwrap_or("")
                } else {
                    ""
                };
                let has_type_param = is_param_list && has_type_param_marker(list_text);
                let bare_elems = is_param_list
                    && !has_type_param
                    && (0..node.child_count()).any(|i| {
                        node.child(i).is_some_and(|c| {
                            !matches!(c.kind(), "(" | ")" | "," | "parameter" | "class_parameter")
                        })
                    });
                if bare_elems || has_type_param {
                    for i in (0..node.child_count()).rev() {
                        if let Some(c) = node.child(i) {
                            stack.push(c);
                        }
                    }
                    continue;
                }
                let multiline = node.start_position().row != node.end_position().row
                    || (lambda && lambda_arrow_on_next_line(&node, s));
                let elem = element_kind(elem_kinds);
                let mut kids = Vec::new();
                for c in node.children(&mut node.walk()) {
                    // Destructuring lambda params (`{ index, (group, items) ->`)
                    // are multi_variable_declaration nodes — the LAST param is
                    // the destructuring group, not the plain variable before it.
                    if c.kind() == elem || (lambda && c.kind() == "multi_variable_declaration") {
                        kids.push(c);
                    }
                }
                if let Some(last) = kids.last() {
                    // For lambda_parameters the closing `->` lives in the
                    // parent lambda_literal, not inside the node — the
                    // generic list_trailing_comma can't see it and wrongly
                    // reports "Missing" (issue #260). Use the last element's
                    // own trailing comma instead.
                    let comma_pos = if lambda {
                        comma_after(last, bytes)
                    } else {
                        list_trailing_comma(&node, bytes)
                    };
                    if comma_pos.is_some() {
                        // Unnecessary: single-line lists always; multiline
                        // under android_studio without the allow flag.
                        if !multiline || self.forbid_trailing_comma {
                            v.push(Violation {
                                file: String::new(),
                                line: comma_pos.unwrap().row + 1,
                                col: comma_pos.unwrap().column + 1,
                                rule_id: self.id().into(),
                                message: format!(
                                    "Unnecessary trailing comma before \"{}\"",
                                    close_msg
                                ),
                                auto_fixable: true,
                            });
                        }
                    } else if multiline && self.require_trailing_comma {
                        // Missing: multiline list that must end with a comma.
                        let pos = last.end_position();
                        v.push(Violation {
                            file: String::new(),
                            line: pos.row + 1,
                            col: pos.column + 1,
                            rule_id: self.id().into(),
                            message: format!("Missing trailing comma before \"{}\"", close_msg),
                            auto_fixable: true,
                        });
                    }
                }
            }
            for i in (0..node.child_count()).rev() {
                if let Some(c) = node.child(i) {
                    stack.push(c);
                }
            }
        }
        v
    }
}

const ELEM_PARAM: u8 = 1;
const ELEM_CLASS_PARAM: u8 = 2;
const ELEM_VAR_DECL: u8 = 3;
const ELEM_ENUM_ENTRY: u8 = 4;

/// True when the text contains a function-type parameter marker — `: (` or
/// `: @Name (` (a plain/annotated function type such as
/// `storyContent: @Composable (character: X) -> Unit`). The marker is what
/// tree-sitter-kotlin-sg mis-parses into separate top-level parameters.
fn has_type_param_marker(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b':' && bytes[i + 1] == b' ' {
            let rest = &text[i + 2..];
            if rest.starts_with('(') {
                return true;
            }
            if let Some(r) = rest.strip_prefix('@') {
                let id_end = r
                    .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
                    .unwrap_or(r.len());
                if r[id_end..].trim_start().starts_with('(') {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

fn element_kind(kind: u8) -> &'static str {
    match kind {
        ELEM_PARAM => "parameter",
        ELEM_CLASS_PARAM => "class_parameter",
        ELEM_VAR_DECL => "variable_declaration",
        ELEM_ENUM_ENTRY => "enum_entry",
        _ => "",
    }
}

/// Byte offset/position of the trailing comma directly after `last` (same
/// line), if any.

/// The trailing comma of a list — the token directly before the closing
/// `)`/`}`/`->`. Works for parameters with multiline default values
/// (`request: Request = url(\n    …,\n),` — the comma sits before `)`).
fn list_trailing_comma(node: &tree_sitter::Node, bytes: &[u8]) -> Option<tree_sitter::Point> {
    // `;` closes an enum body with a semicolon after its entries
    // (`UNKNOWN,` then `;` — the trailing comma sits before `;`, kataris
    // corpus). When a `;` is present (enum semicolon) it wins over the
    // trailing `}`. Otherwise the list's OWN close is the LAST
    // `)`/`}`/`->` child — nested calls inside the list have their own
    // `)` earlier.
    let semicolon = node.children(&mut node.walk()).find(|c| c.kind() == ";");
    let close = match semicolon {
        Some(sc) => sc,
        None => node
            .children(&mut node.walk())
            .filter(|c| matches!(c.kind(), ")" | "}" | "->"))
            .last()?,
    };
    // Last non-whitespace char before the close, skipping comment text and
    // string literals (regular, escaped and raw `"""`) — a trailing
    // comment after the last entry (`Color(0xFFE0CFC2), // peach`), a URL
    // argument (`Uri.parse("kataris:///…")`) or a raw-string argument
    // (`"""<cg url="https://…">…"""`) must not hide the comma.
    let mut i = close.start_byte();
    let mut last = None;
    let mut j = node.start_byte();
    let mut in_line_comment = false;
    let mut block_depth = 0usize;
    let mut in_char = false;
    let mut in_string = false;
    let mut in_raw_string = false;
    while j < i {
        let b = bytes[j];
        if block_depth > 0 {
            // Kotlin block comments NEST — track a depth, not a boolean
            // (reviewer: `/* outer /* inner */ … */`).
            if b == b'/' && j + 1 < i && bytes[j + 1] == b'*' {
                block_depth += 1;
                j += 2;
                continue;
            }
            if b == b'*' && j + 1 < i && bytes[j + 1] == b'/' {
                block_depth -= 1;
                j += 2;
                continue;
            }
            j += 1;
            continue;
        }
        if !in_string
            && !in_raw_string
            && !in_char
            && !in_line_comment
            && b == b'/'
            && j + 1 < i
            && bytes[j + 1] == b'*'
        {
            block_depth = 1;
            j += 2;
            continue;
        }
        if !in_string && !in_raw_string && !in_char && !in_line_comment {
            if b == b'\'' {
                in_char = true;
                j += 1;
                continue;
            }
            if b == b'"' && j + 2 < i && bytes[j + 1] == b'"' && bytes[j + 2] == b'"' {
                in_raw_string = true;
                j += 3;
                continue;
            }
            if b == b'"' {
                in_string = true;
                j += 1;
                continue;
            }
        } else if in_char {
            if b == b'\\' {
                j += 2;
                continue;
            }
            if b == b'\'' {
                in_char = false;
                // Closing the character literal ends the argument — record
                // it so a `"` inside (`'"'`) does not toggle string state
                // (reviewer, round 5).
                last = Some((j, b));
                j += 1;
                continue;
            }
        } else if in_string {
            if b == b'\\' {
                j += 2;
                continue;
            }
            if b == b'"' {
                in_string = false;
                // The closing quote ends the argument — record it so a
                // later scan does not fall back to an EARLIER comma inside
                // the list (kataris KatTheme.kt `Suppress("unused", "…")`).
                last = Some((j, b));
                j += 1;
                continue;
            }
        } else if in_raw_string
            && b == b'"'
            && j + 2 < i
            && bytes[j + 1] == b'"'
            && bytes[j + 2] == b'"'
        {
            in_raw_string = false;
            // Closing triple-quote ends the argument (see the `"` case).
            last = Some((j, b));
            j += 3;
            continue;
        }
        if b == b'\n' {
            in_line_comment = false;
        } else if !in_string
            && !in_raw_string
            && !in_char
            && b == b'/'
            && j + 1 < i
            && bytes[j + 1] == b'/'
        {
            in_line_comment = true;
        }
        if !in_line_comment
            && !in_string
            && !in_raw_string
            && !in_char
            && b != b' '
            && b != b'\t'
            && b != b'\n'
            && b != b'\r'
        {
            last = Some((j, b));
        }
        j += 1;
    }
    if last.map(|(_, b)| b) == Some(b',') {
        let (ci, _) = last.unwrap();
        let line = bytes[..ci].iter().filter(|&&b| b == b'\n').count();
        let line_start = bytes[..ci]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |x| x + 1);
        return Some(tree_sitter::Point {
            row: line,
            column: ci - line_start,
        });
    }
    None
}

fn comma_after(last: &tree_sitter::Node, bytes: &[u8]) -> Option<tree_sitter::Point> {
    let mut i = last.end_byte();
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    if bytes.get(i) == Some(&b',') {
        let line = bytes[..i].iter().filter(|&&b| b == b'\n').count();
        let line_start = bytes[..i]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |x| x + 1);
        return Some(tree_sitter::Point {
            row: line,
            column: i - line_start,
        });
    }
    None
}

/// True when the lambda's `->` sits on a later line than its parameter list
/// (`{ first: Int, second: Int\n    ->`) — the list is then treated as
/// multiline for the trailing-comma check (issue #204).
fn lambda_arrow_on_next_line(params: &tree_sitter::Node, source: &str) -> bool {
    let mut stack = vec![params.parent()];
    while let Some(node) = stack.pop() {
        if let Some(n) = node {
            if n.kind() == "lambda_literal" {
                let text = &source[n.start_byte()..n.end_byte()];
                return text[params.end_byte() - n.start_byte()..]
                    .find("->")
                    .is_some_and(|off| {
                        let arrow_abs = params.end_byte() + off;
                        source[..arrow_abs].bytes().filter(|&b| b == b'\n').count()
                            > source[..params.end_byte()]
                                .bytes()
                                .filter(|&b| b == b'\n')
                                .count()
                    });
            }
            stack.push(n.parent());
        }
    }
    false
}

pub struct TrailingCommaOnCallSite {
    require_trailing_comma: bool,
    forbid_trailing_comma: bool,
}

impl TrailingCommaOnCallSite {
    pub fn new(allow_trailing_comma: bool, is_android_studio: bool) -> Self {
        Self {
            require_trailing_comma: allow_trailing_comma || !is_android_studio,
            forbid_trailing_comma: is_android_studio && !allow_trailing_comma,
        }
    }
}

impl Rule for TrailingCommaOnCallSite {
    fn id(&self) -> &'static str {
        "standard:trailing-comma-on-call-site"
    }
    fn check(&self, tree: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let bytes = s.as_bytes();
        let mut v = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "value_arguments" {
                // A function-type argument list mis-parses as value_arguments
                // (`onOpenCoverMedia: ((url: String, …) -> Unit)?` — the
                // type's `(` opener is preceded by `: ` / `@`). Its trailing
                // comma is not measurable; oracle stays silent.
                let opener = node.start_byte();
                let type_paren = opener > 0
                    && (0..opener)
                        .rev()
                        .find(|&j| !matches!(bytes[j], b' ' | b'\t'))
                        .is_some_and(|j| bytes[j] == b':' || bytes[j] == b'@');
                if type_paren {
                    for i in (0..node.child_count()).rev() {
                        if let Some(c) = node.child(i) {
                            stack.push(c);
                        }
                    }
                    continue;
                }
                let multiline = node.start_position().row != node.end_position().row;
                let mut kids = Vec::new();
                for c in node.children(&mut node.walk()) {
                    if c.kind() == "value_argument" {
                        kids.push(c);
                    }
                }
                if let Some(last) = kids.last() {
                    // A list whose only argument is a lambda
                    // (`withTransform({\n … })` — anonymous, or
                    // `scaleClickable(onClick = {\n … })` — named) needs no
                    // trailing comma (oracle clean).
                    let single_lambda = kids.len() == 1
                        && s[last.start_byte()..last.end_byte()]
                            .trim_end()
                            .ends_with('}');
                    let comma_pos = list_trailing_comma(&node, bytes);
                    if comma_pos.is_some() {
                        if !multiline || self.forbid_trailing_comma {
                            v.push(Violation {
                                file: String::new(),
                                line: comma_pos.unwrap().row + 1,
                                col: comma_pos.unwrap().column + 1,
                                rule_id: self.id().into(),
                                message: "Unnecessary trailing comma before \")\"".into(),
                                auto_fixable: true,
                            });
                        }
                    } else if multiline && self.require_trailing_comma && !single_lambda {
                        // A lambda as the LAST argument with the FIRST
                        // argument on the opener line (`MessageAvatar(item,

                        // onClick = { … })`) is a trailing-lambda style —
                        // oracle stays silent; only a fully multiline list
                        // (first arg on its own line) demands the comma.
                        let first_on_open_line = kids
                            .first()
                            .is_some_and(|f| f.start_position().row == node.start_position().row);
                        let last_is_lambda = s[last.start_byte()..last.end_byte()]
                            .trim_end()
                            .ends_with('}');
                        if !(first_on_open_line && last_is_lambda) {
                            let pos = last.end_position();
                            v.push(Violation {
                                file: String::new(),
                                line: pos.row + 1,
                                col: pos.column + 1,
                                rule_id: self.id().into(),
                                message: "Missing trailing comma before \")\"".into(),
                                auto_fixable: true,
                            });
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
        v
    }
}

#[cfg(test)]
mod trailing_comma_lambda_tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(source);
        TrailingCommaOnDeclarationSite {
            require_trailing_comma: true,
            forbid_trailing_comma: false,
        }
        .check(&tree, source)
    }

    // Issue #260: a multiline lambda parameter list whose last parameter
    // already ends with a trailing comma must not report "Missing".
    #[test]
    fn multiline_lambda_with_trailing_comma_is_clean() {
        let src = "package com.example\n\npublic class FunctionLiteral {\n    public fun bind() {\n        val latest = rememberUpdatedState<(String, String) -> Unit> {\n                exampleEmail,\n                examplePassword,\n            ->\n            handle(exampleEmail, examplePassword)\n        }\n        use(latest)\n    }\n}\n";
        assert!(check(src).is_empty());
    }

    #[test]
    fn multiline_lambda_without_trailing_comma_reports_missing() {
        // oracle L5: typed lambda params with the arrow on its own line and
        // no trailing comma report "Missing". (Untyped params — L4 — are a
        // pre-existing ktlint-rs blind spot, out of scope.)
        let src = "package com.example\n\npublic class Test {\n    public fun a() {\n        val f = {\n            exampleEmail: String\n            ->\n            use(exampleEmail)\n        }\n        use(f)\n    }\n}\n";
        let v = check(src);
        assert!(
            v.iter()
                .any(|x| x.message.contains("Missing trailing comma")),
            "violations: {:?}",
            v.iter().map(|x| &x.message).collect::<Vec<_>>()
        );
    }
}

#[cfg(test)]
mod trailing_comma_regression_tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn decl_check(src: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(src);
        TrailingCommaOnDeclarationSite {
            require_trailing_comma: true,
            forbid_trailing_comma: false,
        }
        .check(&tree, src)
    }

    fn call_check(src: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(src);
        TrailingCommaOnCallSite {
            require_trailing_comma: true,
            forbid_trailing_comma: false,
        }
        .check(&tree, src)
    }

    // kataris: destructuring lambda params ({ index, (group, items) -> }) —
    // the destructuring group is the LAST param; no trailing comma present.
    #[test]
    fn lambda_destructuring_no_trailing_comma_clean() {
        let src = "package com.example\n\nfun main() {\n    val grouped = listOf(1 to 2)\n    grouped.forEachIndexed { groupIndex, (group, items) ->\n        println(\"$groupIndex $group $items\")\n    }\n}\n";
        assert!(decl_check(src).is_empty());
    }

    // kataris: function-type param mis-parses — trailing comma present.
    #[test]
    fn function_type_param_misparse_clean() {
        let src = "package com.example\n\nfun HomeShelfSection(\n    storyContent: @Composable (character: HomeCharacter, modifier: Modifier) -> Unit,\n) {\n    use(storyContent)\n}\n";
        assert!(decl_check(src).is_empty());
    }

    // kataris: enum body with a semicolon after the entries.
    #[test]
    fn enum_with_semicolon_clean() {
        let src = "package com.example\n\nenum class CreatorStudioGenerationStatus {\n    PENDING,\n    UNKNOWN,\n    ;\n}\n";
        assert!(decl_check(src).is_empty());
    }

    // kataris: single anonymous lambda argument ({ … }).
    #[test]
    fn single_lambda_argument_clean() {
        let src = "package com.example\n\nfun main() {\n    withTransform({\n        translate(1f)\n    }) {\n        drawPath()\n    }\n}\n";
        assert!(call_check(src).is_empty());
    }

    // kataris: named lambda last arg with first arg on opener line.
    #[test]
    fn first_arg_on_open_line_with_lambda_clean() {
        let src = "package com.example\n\nfun main() {\n    MessageAvatar(item, onClick = {\n        onMarkRead()\n    })\n}\n";
        assert!(call_check(src).is_empty());
    }

    // kataris: trailing comment must not hide the existing comma.
    #[test]
    fn trailing_comment_keeps_existing_comma() {
        let src = "package com.example\n\nval colors = listOf(\n    Color(0xFFDED6CB), // taupe\n    Color(0xFFE0CFC2), // peach\n)\n";
        assert!(call_check(src).is_empty());
    }

    // kataris: a URL inside an argument must not hide the comma or trigger
    // a false "Missing" (nested multiline call).
    #[test]
    fn nested_call_with_url_clean() {
        let src = "package com.example\n\nfun main() {\n    assertEquals(\n        home(HomeDeepLinkTarget(homeTabId = \"new\")),\n        DeepLinkParser.parse(Uri.parse(\"kataris:///home?tab=new\"), hosts),\n    )\n}\n";
        assert!(call_check(src).is_empty());
    }

    // A fully-multiline list still demands the trailing comma (oracle C8).
    #[test]
    fn fully_multiline_still_reports_missing() {
        let src =
            "package com.example\n\nfun main() {\n    foo(\n        a,\n        b\n    )\n}\n";
        assert!(
            call_check(src)
                .iter()
                .any(|x| x.message.contains("Missing trailing comma")),
            "fully-multiline list must report Missing"
        );
    }

    // Missing trailing comma in a DECL parameter list still reports.
    #[test]
    fn decl_missing_comma_still_reports() {
        let src =
            "package com.example\n\nfun g(\n    a: Int,\n    b: String\n) {\n    use(a, b)\n}\n";
        assert!(
            decl_check(src)
                .iter()
                .any(|x| x.message.contains("Missing trailing comma")),
            "decl list without trailing comma must report"
        );
    }
}

#[cfg(test)]
mod tc_nested_comment_tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(src);
        TrailingCommaOnDeclarationSite {
            require_trailing_comma: true,
            forbid_trailing_comma: false,
        }
        .check(&tree, src)
    }

    // Reviewer round 4: Kotlin block comments NEST — the trailing comma
    // after `arg, /* outer /* inner */ … */` must still be found.
    #[test]
    fn nested_block_comment_keeps_trailing_comma() {
        let src = "package com.example\n\nfun f(\n    a: Int,\n    b: Int, /* outer /* inner */ explanation */\n) {\n    use(a, b)\n}\n";
        assert!(check(src).is_empty());
    }
}

#[cfg(test)]
mod tc_char_literal_tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(src);
        TrailingCommaOnDeclarationSite {
            require_trailing_comma: true,
            forbid_trailing_comma: false,
        }
        .check(&tree, src)
    }

    // Reviewer round 5: a character literal containing a double quote
    // ('"') must not toggle the string state — the trailing comma after it
    // stays visible.
    #[test]
    fn char_literal_with_quote_keeps_trailing_comma() {
        let src = "package com.example\n\nenum class Quotes {\n    DOUBLE_QUOTE,\n    SINGLE_QUOTE,\n    ;\n    val q = '\"'\n}\n";
        assert!(check(src).is_empty());
        let src2 = "package com.example\n\nfun f(\n    a: Char = '\\'',\n    b: Char = '\"',\n) {\n    use(a, b)\n}\n";
        assert!(check(src2).is_empty());
    }
}
