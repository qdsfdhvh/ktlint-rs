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
    /// Oracle matrix: ktlint_official / intellij_idea always demand the
    /// trailing comma on multiline declaration-site lists; android_studio
    /// only does under `ij_kotlin_allow_trailing_comma = true`.
    require_trailing_comma: bool,
}

impl TrailingCommaOnDeclarationSite {
    pub fn new(allow_trailing_comma: bool, is_android_studio: bool) -> Self {
        Self {
            require_trailing_comma: allow_trailing_comma || !is_android_studio,
        }
    }
}

impl Rule for TrailingCommaOnDeclarationSite {
    fn id(&self) -> &'static str {
        "standard:trailing-comma-on-declaration-site"
    }
    fn check(&self, tree: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = s.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            let is_lambda_params = node.kind() == "lambda_parameters";
            let multiline = node.start_position().row != node.end_position().row
                || (is_lambda_params && lambda_arrow_on_next_line(&node, s));
            if matches!(
                node.kind(),
                "function_value_parameters" | "class_parameters" | "lambda_parameters"
            ) && multiline
            {
                // multiline list: the last parameter must be followed by a
                // trailing comma (issue #204).
                let mut w = node.walk();
                let kids: Vec<tree_sitter::Node> = node.children(&mut w).collect();
                let params: Vec<&tree_sitter::Node> = kids
                    .iter()
                    .filter(|c| {
                        matches!(
                            c.kind(),
                            "parameter" | "class_parameter" | "variable_declaration"
                        )
                    })
                    .collect();
                if let Some(last) = params.last() {
                    let line_start = bytes[..last.end_byte()]
                        .iter()
                        .rposition(|&b| b == b'\n')
                        .map_or(0, |i| i + 1);
                    let line_end = bytes[last.end_byte()..]
                        .iter()
                        .position(|&b| b == b'\n')
                        .map_or(bytes.len(), |i| last.end_byte() + i);
                    let trimmed = bytes[line_start..line_end]
                        .iter()
                        .rposition(|&b| b != b' ' && b != b'\t')
                        .map_or(line_start, |i| line_start + i);
                    let has_comma = bytes.get(trimmed) == Some(&b',');
                    // Oracle: ktlint_official always demands the trailing
                    // comma; android_studio only under
                    // `ij_kotlin_allow_trailing_comma = true`. (The
                    // "unnecessary trailing comma" direction depends on the
                    // enclosing class context in ktlint 1.8 and is left
                    // unreported for now.)
                    if self.require_trailing_comma && !has_comma {
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
            let mut w = node.walk();
            let kids: Vec<tree_sitter::Node> = node.children(&mut w).collect();
            for k in kids.into_iter().rev() {
                stack.push(k);
            }
        }
        v
    }
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

pub struct TrailingCommaOnCallSite;
impl Rule for TrailingCommaOnCallSite {
    fn id(&self) -> &'static str {
        "standard:trailing-comma-on-call-site"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.contains("(")
                && l.contains(")")
                && !l.trim().starts_with("fun ")
                && !l.trim().starts_with("class ")
            {
                if let Some(rp) = l.rfind(')') {
                    if rp > 1 && l.as_bytes()[rp - 1] == b',' {
                        v.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: rp + 1,
                            rule_id: self.id().into(),
                            message: "Missing trailing comma on call site".into(),
                            auto_fixable: true,
                        });
                    }
                }
            }
        }
        v
    }
}
