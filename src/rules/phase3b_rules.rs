//! Phase 3.3 semantics: no-empty-file, indent, max-line, kdoc, function-signature
use crate::config::CodeStyle;
use crate::rules::{Rule, Violation};

pub struct FunctionSignatureSpacing {
    max_length: usize,
    code_style: crate::config::CodeStyle,
}

impl FunctionSignatureSpacing {
    pub fn new(max_length: usize, code_style: crate::config::CodeStyle) -> Self {
        let max_length = if max_length == 0 { 120 } else { max_length };
        Self {
            max_length,
            code_style,
        }
    }

    /// Body-expression merge (mirrors ktlint 1.8 FunctionSignatureRule):
    /// `fun foo(...): Type =` followed by a newline + body expression. When
    /// the first line of the body fits on the signature line
    /// (`firstLineOfBodyExpression.length < maxLineLength - signatureLength`,
    /// strict), the body should join the signature.
    fn check_body_merge(&self, tree: &tree_sitter::Tree, s: &str, v: &mut Vec<Violation>) {
        let max_length = self.max_length;
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "function_declaration" {
                if let Some(body) = node
                    .children(&mut node.walk())
                    .find(|c| c.kind() == "function_body")
                {
                    let mut body_walker = body.walk();
                    let mut it = body.children(&mut body_walker);
                    let eq = it.next();
                    if let Some(eq) = eq.filter(|n| n.kind() == "=") {
                        let Some(expr) = it.next() else { continue };
                        // Body must be on a later line than the `=` (newline between).
                        if expr.start_position().row <= eq.start_position().row {
                            continue;
                        }
                        // Signature length mirrors ktlint's
                        // `indent + functionSignatureNodes.joinTextToString()`:
                        // every leaf from the first code token to `=` (end
                        // inclusive) with whitespace preserved — i.e. the raw
                        // text of the whole signature as if on one line
                        // (multiline signatures keep their newlines, which makes
                        // them too long to merge).
                        let func_start = node.start_byte();
                        let indent_start = s[..func_start].rfind('\n').map_or(0, |i| i + 1);
                        let indent_len = s[indent_start..func_start].chars().count();
                        let sig_len = indent_len + s[func_start..eq.end_byte()].chars().count();
                        // ktlint only rewrites an over-long signature to
                        // multiline (then measures from the last signature line,
                        // `maxLengthRemaining = maxLineLength - lengthOfLastLine`)
                        // when the signature *has parameters*. A parameterless
                        // multiline signature is never rewritten — the full
                        // (multiline) signature length is used, which exceeds the
                        // limit and therefore never merges the body.
                        let params_node = node
                            .children(&mut node.walk())
                            .find(|c| c.kind() == "function_value_parameters");
                        let param_multiline = params_node.is_some_and(|p| {
                            p.utf8_text(s.as_bytes()).is_ok_and(|t| t.contains('\n'))
                        });
                        let has_params = params_node.is_some_and(|p| {
                            let mut w = p.walk();
                            let any = p.children(&mut w).any(|c| c.kind() == "value_parameter");
                            any
                        });
                        let remaining = if has_params && (param_multiline || sig_len > max_length) {
                            max_length.saturating_sub(eq.end_position().column)
                        } else {
                            max_length.saturating_sub(sig_len)
                        };
                        // First line of body expression (no leading indent —
                        // expression node starts at first code token).
                        let body_start = expr.start_byte();
                        let body_line_end = s[body_start..]
                            .find('\n')
                            .map_or(s.len(), |i| body_start + i);
                        let first_line = &s[body_start..body_line_end];
                        let first_line_len = first_line.len();
                        // Never merge an annotated expression body.
                        if first_line.trim_start().starts_with('@') {
                            continue;
                        }
                        // Never merge a multiline string template body.
                        if s[body_start..expr.end_byte()].contains("\"\"\"") {
                            continue;
                        }
                        // A multiline expression body (e.g. a `when` or `if`
                        // chain) must stay on its own line under
                        // ktlint_official — multiline-expression-wrapping
                        // demands it, so "fits on same line" never applies
                        // (issue #160). Under android_studio ktlint reports it
                        // (issue #167).
                        if expr.start_position().row != expr.end_position().row
                            && self.code_style == crate::config::CodeStyle::KtlintOfficial
                        {
                            continue;
                        }
                        // Strictly less than the remaining space.
                        if first_line_len >= remaining {
                            continue;
                        }
                        v.push(Violation {
                            file: String::new(),
                            line: eq.start_position().row + 1,
                            col: eq.end_position().column + 1,
                            rule_id: self.id().into(),
                            message:
                                "First line of body expression fits on same line as function signature"
                                    .into(),
                            auto_fixable: true,
                        });
                    }
                }
            }
            let mut walker = node.walk();
            for c in node.children(&mut walker) {
                stack.push(c);
            }
        }
    }
}

impl Rule for FunctionSignatureSpacing {
    fn id(&self) -> &'static str {
        "standard:function-signature"
    }
    fn check(&self, tree: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        self.check_multiline_parameters(tree, s, &mut v);
        self.check_body_merge(tree, s, &mut v);
        v
    }
}

impl FunctionSignatureSpacing {
    /// Issue #184: a multiline value parameter list is reported the same way
    /// ktlint's function-signature does — the single-line form is demanded
    /// when it exists:
    /// - a *single* parameter split across lines always collapses (no
    ///   max_line_length check), under both code styles;
    /// - *multiple* parameters collapse only under android_studio and only
    ///   when the collapsed signature fits max_line_length;
    /// - an *empty* list split across lines reports "No whitespace expected
    ///   in empty parameter list" under both styles.
    fn check_multiline_parameters(
        &self,
        tree: &tree_sitter::Tree,
        s: &str,
        v: &mut Vec<Violation>,
    ) {
        let bytes = s.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "function_declaration" {
                self.check_params(&node, bytes, v);
            }
            let mut w = node.walk();
            let kids: Vec<tree_sitter::Node> = node.children(&mut w).collect();
            for k in kids.into_iter().rev() {
                stack.push(k);
            }
        }
    }

    fn check_params(&self, node: &tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
        let Some(params) = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "function_value_parameters")
        else {
            return;
        };
        if params.start_position().row == params.end_position().row {
            return;
        }
        let mut w = params.walk();
        let param_nodes: Vec<tree_sitter::Node> = params
            .children(&mut w)
            .filter(|c| c.kind() == "parameter")
            .collect();
        if param_nodes.is_empty() {
            // `fun f(\n)` — ktlint reports at the column right after `(`.
            let pos = params.start_position();
            v.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 2,
                rule_id: self.id().to_string(),
                message: "No whitespace expected in empty parameter list".into(),
                auto_fixable: true,
            });
            return;
        }
        // ktlint_official never collapses a *single* annotated parameter
        // (`@TempDir tempDir: Path` keeps its line) — android_studio does.
        let annotated_param = {
            let mut w = params.walk();
            let kids: Vec<tree_sitter::Node> = params.children(&mut w).collect();
            let idx = kids.iter().position(|c| c.kind() == "parameter");
            idx.is_some_and(|i| i > 0 && kids[i - 1].kind() == "parameter_modifiers")
        };
        let single = param_nodes.len() == 1
            && self.single_line_param(&params, &param_nodes[0], bytes)
            && !(self.code_style != CodeStyle::AndroidStudio && annotated_param);
        let multi = param_nodes.len() > 1;
        // Issue #194: a single parameter list must respect max_line_length
        // too — ktlint stops asking for the collapse once the collapsed
        // signature (incl. the ` {`) reaches 120 (oracle-verified).
        let fits = self.signature_fits(node, &params, bytes);
        let single_fits = self.signature_fits_single(node, &params, bytes);
        if !((single && single_fits)
            || (multi && self.code_style == CodeStyle::AndroidStudio && fits))
        {
            return;
        }
        // ktlint reports the first parameter at its first token, including
        // parameter modifiers (`@TempDir tempDir: Path` reports at `@`) —
        // but only when the first parameter starts on its own line. A
        // parameter on the opening-paren line (`fun f(alpha: String,` shape,
        // handled by parameter-list-wrapping) gets no "first parameter"
        // message.
        let first_on_new_line = param_nodes[0].start_position().row > params.start_position().row;
        let first_pos = {
            let mut w = params.walk();
            let kids: Vec<tree_sitter::Node> = params.children(&mut w).collect();
            let idx = kids.iter().position(|c| c.kind() == "parameter");
            idx.and_then(|i| kids.get(i.saturating_sub(1)))
                .filter(|c| c.kind() == "parameter_modifiers")
                .map_or_else(|| param_nodes[0].start_position(), |c| c.start_position())
        };
        for (idx, p) in param_nodes.iter().enumerate() {
            let pos = if idx == 0 {
                first_pos
            } else {
                p.start_position()
            };
            let message = if idx == 0 {
                if !first_on_new_line {
                    continue;
                }
                "No whitespace expected between opening parenthesis and first parameter name"
            } else {
                "Single whitespace expected before parameter"
            };
            v.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: self.id().to_string(),
                message: message.to_string(),
                auto_fixable: true,
            });
        }
        // Last parameter's line end — only when the line ends with a comma
        // (a parameter on the closing-paren line gets no message; ktlint
        // reports the column after the trailing comma).
        if let Some(last) = param_nodes.last() {
            let row = last.end_position().row;
            let line_start = bytes[..last.end_byte()]
                .iter()
                .rposition(|&b| b == b'\n')
                .map_or(0, |i| i + 1);
            let line_end = bytes[line_start..]
                .iter()
                .position(|&b| b == b'\n')
                .map_or(bytes.len(), |i| line_start + i);
            let line_text = &bytes[line_start..line_end];
            let trimmed = line_text
                .iter()
                .rposition(|&b| b != b' ' && b != b'\t')
                .map_or(line_start, |i| line_start + i);
            // Report only when the closing `)` is NOT on the parameter's own
            // line (`beta: String)` or `beta: String) = Unit` keep it).
            let rest_has_close = bytes[last.end_byte()..line_end].contains(&b')');
            if !rest_has_close {
                v.push(Violation {
                    file: String::new(),
                    line: row + 1,
                    col: trimmed - line_start + 2,
                    rule_id: self.id().to_string(),
                    message:
                        "No whitespace expected between last parameter and closing parenthesis"
                            .to_string(),
                    auto_fixable: true,
                });
            }
        }
    }

    /// Whether a (single) parameter occupies exactly one line including its
    /// default value — `a: Int = 1,` yes, `request: Request =\n      Request…`
    /// no (a multiline default expression keeps the list multiline even
    /// though the collapsed form would fit, matching ktlint).
    fn single_line_param(
        &self,
        params: &tree_sitter::Node,
        param: &tree_sitter::Node,
        bytes: &[u8],
    ) -> bool {
        let mut w = params.walk();
        let kids: Vec<tree_sitter::Node> = params.children(&mut w).collect();
        let start = param.start_byte();
        let end = kids
            .iter()
            .skip_while(|c| c.start_byte() < start)
            .find(|c| c.kind() == "," || c.kind() == ")")
            .map_or(params.end_byte(), |c| c.end_byte());
        !bytes[start..end].contains(&b'\n')
    }

    /// Single-parameter variant of the fits check. Oracle boundary: the
    /// collapsed signature (indent + params + ` {`) collapses up to 120,
    /// silent at 121 — one column later than the multi-parameter check
    /// (issue #195).
    fn signature_fits_single(
        &self,
        node: &tree_sitter::Node,
        params: &tree_sitter::Node,
        bytes: &[u8],
    ) -> bool {
        let start = self.measure_start(node, bytes);
        let byte = params.end_byte();
        let line_end = bytes[byte..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(bytes.len(), |i| byte + i);
        let mut e = line_end;
        while e > byte && (bytes[e - 1] == b' ' || bytes[e - 1] == b'\t') {
            e -= 1;
        }
        if e <= start {
            return false;
        }
        let text = match std::str::from_utf8(&bytes[start..e]) {
            Ok(t) => t,
            Err(_) => return false,
        };
        let decl_start = node.start_byte();
        let line_start = bytes[..decl_start]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        let indent_len = bytes[line_start..decl_start]
            .iter()
            .filter(|&&b| b == b' ' || b == b'\t')
            .count();
        let collapsed_len = {
            let lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();
            let mut len = 0usize;
            for (i, l) in lines.iter().enumerate() {
                len += l.chars().count();
                if i + 1 < lines.len() && l.ends_with(',') && !lines[i + 1].starts_with(')') {
                    len += 1;
                }
            }
            len
        };
        indent_len + collapsed_len <= self.max_length
    }

    /// Whether the collapsed signature (from the first non-annotation
    /// modifier, or `fun`, to the parameter list's closing paren) fits within
    /// max_line_length. Lines are trimmed and joined with no separator,
    /// mirroring ktlint's own measurement (same convention as
    /// class-signature, issue #182).
    fn signature_fits(
        &self,
        node: &tree_sitter::Node,
        params: &tree_sitter::Node,
        bytes: &[u8],
    ) -> bool {
        let start = self.measure_start(node, bytes);
        // Measure to the end of the closing-paren line: ` {`, `: Int {` etc.
        // on that line count against max_line_length too (issue #188).
        let end = {
            let byte = params.end_byte();
            let line_end = bytes[byte..]
                .iter()
                .position(|&b| b == b'\n')
                .map_or(bytes.len(), |i| byte + i);
            let mut e = line_end;
            while e > byte && (bytes[e - 1] == b' ' || bytes[e - 1] == b'\t') {
                e -= 1;
            }
            e
        };
        if end <= start {
            return false;
        }
        let text = match std::str::from_utf8(&bytes[start..end]) {
            Ok(t) => t,
            Err(_) => return false,
        };
        let decl_start = node.start_byte();
        let line_start = bytes[..decl_start]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        let indent_len = bytes[line_start..decl_start]
            .iter()
            .filter(|&&b| b == b' ' || b == b'\t')
            .count();
        // ktlint's collapsed single-line form: lines trimmed, joined with
        // `, ` after a trailing comma (`fun f(\n    a: Int,\n    b: Int)`
        // -> `fun f(a: Int, b: Int)`).
        let collapsed_len = {
            let lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();
            let mut len = 0usize;
            for (i, l) in lines.iter().enumerate() {
                len += l.chars().count();
                if i + 1 < lines.len() && l.ends_with(',') && !lines[i + 1].starts_with(')') {
                    len += 1;
                }
            }
            len
        };
        indent_len + collapsed_len <= self.max_length
    }

    /// Byte offset where the signature measurement starts: the first
    /// non-annotation token of the `modifiers` node, else the `fun` keyword.
    fn measure_start<'a>(&self, node: &tree_sitter::Node<'a>, bytes: &[u8]) -> usize {
        if let Some(mods) = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "modifiers")
        {
            let text = &bytes[mods.start_byte()..mods.end_byte()];
            let mut i = 0usize;
            while i < text.len() && text[i] == b'@' {
                i += 1;
                while i < text.len()
                    && (text[i].is_ascii_alphanumeric() || text[i] == b'_' || text[i] == b'.')
                {
                    i += 1;
                }
                if i < text.len() && text[i] == b'(' {
                    let mut depth = 1usize;
                    i += 1;
                    while i < text.len() && depth > 0 {
                        match text[i] {
                            b'(' => depth += 1,
                            b')' => depth = depth.saturating_sub(1),
                            _ => {}
                        }
                        i += 1;
                    }
                }
                while i < text.len() && text[i].is_ascii_whitespace() {
                    i += 1;
                }
            }
            if i < text.len() {
                return mods.start_byte() + i;
            }
        }
        node.children(&mut node.walk())
            .find(|c| c.kind() == "fun")
            .map_or(node.start_byte(), |k| k.start_byte())
    }
}

pub struct FunctionExpressionBody;
impl Rule for FunctionExpressionBody {
    fn id(&self) -> &'static str {
        "standard:function-expression-body"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let lines: Vec<&str> = s.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let t = lines[i].trim();
            // Find block-body functions: fun name(...): Type {
            if t.starts_with("fun ") && t.ends_with('{') && !t.contains('=') {
                let _fun_line = i;
                i += 1;
                let mut depth = 1usize;
                let mut return_count = 0usize;
                let mut return_line = 0usize;
                let mut has_other_statements = false;
                while i < lines.len() && depth > 0 {
                    let body = lines[i].trim();
                    let opens = body.matches('{').count();
                    let closes = body.matches('}').count();
                    depth = depth + opens - closes;
                    if body.starts_with("return ") && !body.contains("//") {
                        return_count += 1;
                        return_line = i;
                    } else if !body.is_empty() && !body.starts_with("//") && body != "}" {
                        // Check if it's a real statement (not just a closing brace line)
                        if closes == 0 || !body.trim_end_matches('}').trim().is_empty() {
                            has_other_statements = true;
                        }
                    }
                    i += 1;
                }
                // Flag if exactly one return and no other statements in body
                if return_count == 1 && !has_other_statements {
                    v.push(Violation {
                        file: String::new(),
                        line: return_line + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Function body should be replaced with body expression".into(),
                        auto_fixable: true,
                    });
                }
            } else {
                i += 1;
            }
        }
        v
    }
}

pub struct KeywordSpacing;
impl Rule for KeywordSpacing {
    fn id(&self) -> &'static str {
        "standard:keyword-spacing"
    }
    fn auto_fixable(&self) -> bool {
        true
    }
    fn check(&self, tree: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let bytes = s.as_bytes();
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if matches!(
                node.kind(),
                "if" | "for" | "while" | "when" | "try" | "catch"
            ) && bytes.get(node.end_byte()) == Some(&b'(')
            {
                let pos = node.start_position();
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 3,
                    rule_id: self.id().into(),
                    message: format!("Missing spacing after \"{}\"", node.kind()),
                    auto_fixable: true,
                });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn keyword_check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        KeywordSpacing.check(&tree, source)
    }

    #[test]
    fn keyword_spacing_covers_control_and_catch_keywords() {
        assert_eq!(keyword_check("if(true) println(1)\n").len(), 1);
        assert_eq!(
            keyword_check("try { run() } catch(e: E) { fail() }\n").len(),
            1
        );
        assert!(keyword_check("if (true) println(1)\n").is_empty());
    }

    fn fs_check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        FunctionSignatureSpacing::new(120, crate::config::CodeStyle::KtlintOfficial)
            .check(&tree, source)
    }

    // Issue #184: multiline value parameter lists demand the single-line
    // form when ktlint does.
    fn fn_check(src: &str, style: crate::config::CodeStyle) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(src);
        FunctionSignatureSpacing::new(120, style).check(&tree, src)
    }

    #[test]
    fn single_param_multiline_reports_collapse() {
        let src = "fun f(\n    a: Int,\n) {\n}\n";
        let v = fn_check(src, crate::config::CodeStyle::AndroidStudio);
        assert_eq!(
            v.len(),
            2,
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
        assert!(v.iter().any(|x| x.message.contains("first parameter name")));
        assert!(v
            .iter()
            .any(|x| x.message.contains("last parameter and closing")));
    }

    #[test]
    #[test]
    fn single_param_multiline_reports_under_official_too() {
        let src = "fun f(\n    a: Int,\n) {\n}\n";
        let v = fn_check(src, crate::config::CodeStyle::KtlintOfficial);
        assert_eq!(v.len(), 2);
    }

    #[test]
    #[test]
    fn multi_param_collapse_is_android_studio_only() {
        let src = "fun f(\n    a: Int,\n    b: String,\n) {\n}\n";
        let android = fn_check(src, crate::config::CodeStyle::AndroidStudio);
        assert_eq!(android.len(), 3, "3 messages under android_studio");
        let official = fn_check(src, crate::config::CodeStyle::KtlintOfficial);
        assert!(
            official.is_empty(),
            "multi-param multiline is legal under ktlint_official"
        );
    }

    #[test]
    #[test]
    fn annotated_single_param_exempt_under_official() {
        let src = "fun setup(\n    @TempDir tempDir: Path,\n) {\n}\n";
        let official = fn_check(src, crate::config::CodeStyle::KtlintOfficial);
        assert!(
            official.is_empty(),
            "annotated single param stays multiline under official"
        );
        let android = fn_check(src, crate::config::CodeStyle::AndroidStudio);
        assert_eq!(android.len(), 2, "android_studio still collapses it");
    }

    #[test]
    #[test]
    fn empty_split_list_reports() {
        let src = "fun f(\n) {\n}\n";
        for style in [
            crate::config::CodeStyle::AndroidStudio,
            crate::config::CodeStyle::KtlintOfficial,
        ] {
            let v = fn_check(src, style);
            assert!(
                v.iter().any(|x| x.message.contains("empty parameter list")),
                "violations: {:?}",
                v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    // Issue #194: a single-parameter list respects max_line_length too —
    // the collapsed signature (incl. the ` {`) is silent at 120+.
    #[test]
    fn single_param_too_long_stays_multiline() {
        let src = "package com.example\n\nprivate fun exampleFunctionxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx(\n    parameterName: String,\n) {\n    TODO()\n}\n";
        let v = fn_check(src, crate::config::CodeStyle::AndroidStudio);
        assert!(
            v.is_empty(),
            "collapsed ~160 cols must not be collapsed: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    // Issue #195: fn boundary — collapsed-to-')' (incl. trailing comma and
    // the ` {`) collapses up to 120, silent at 121 (oracle-verified).
    #[test]
    fn fn_collapse_boundary_120() {
        // x=36 → collapsed-to-')' = 120
        let fits = "package com.example\n\nprivate fun exampleFunctionxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx(\n    firstParameterName: String,\n    secondParameterName: String,\n) {\n    TODO()\n}\n";
        // x=37 → collapsed-to-')' = 121
        let too_long = "package com.example\n\nprivate fun exampleFunctionxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx(\n    firstParameterName: String,\n    secondParameterName: String,\n) {\n    TODO()\n}\n";
    }

    #[test]
    fn multiline_default_value_keeps_list_multiline() {
        let src = "private fun newWebSocket(\n    request: Request =\n      Request\n        .Builder()\n        .url(\n            \"ws://example.com\"\n        ),\n) {\n}\n";
        let v = fn_check(src, crate::config::CodeStyle::AndroidStudio);
        assert!(v.is_empty(), "multiline default keeps the list multiline");
    }

    #[test]
    fn multiline_body_never_reports_fits_on_same_line() {
        // Issue #160: a multiline expression body (`when { ... }`) must stay
        // on its own line — ktlint's multiline-expression-wrapping demands it,
        // so function-signature must not ask to merge it.
        let src = "package com.example\n\nclass Plain {\n    private val a: Int? = null\n\n    fun render(): String =\n        when {\n            a != null -> \"A\"\n            else -> \"C\"\n        }\n}\n";
        let v = fs_check(src);
        let fs: Vec<_> = v
            .iter()
            .filter(|x| x.message.contains("First line of body expression"))
            .collect();
        assert!(fs.is_empty(), "multiline body must not report fits: {fs:?}");
    }

    fn body_merge_reports_when_body_fits_on_signature_line() {
        // Case B from #101: `buildString(` fits after the signature. The body
        // must be single-line — a multiline body (`mapOf(\n ... \n)`) stays on
        // its own line, matching ktlint 1.8 (multiline-expression-wrapping
        // demands it; issue #160).
        let src = "package com.example\n\nfun build(extra: Array<Pair<String, String>>): Map<String, String> =\n    buildString()\n";
        let v = fs_check(src);
        let fs: Vec<_> = v
            .iter()
            .filter(|x| x.message.contains("First line of body expression"))
            .collect();
        assert_eq!(fs.len(), 1);
        assert_eq!(fs[0].line, 3);
        assert_eq!(fs[0].col, 69); // offset right after the `=`
    }

    #[test]
    fn body_merge_skips_when_body_does_not_fit() {
        // Signature incl. `=` = 75 chars -> 45 remaining; body is 46 -> no merge.
        let src = "package com.example\n\nclass Test {\n    override suspend fun currentLastSocialSignInProvider(): AuthProvider? =\n        sessionStore.currentLastSocialSignInProvider()\n}\n";
        let v = fs_check(src);
        assert!(
            v.iter()
                .filter(|x| x.message.contains("First line of body expression"))
                .next()
                .is_none(),
            "46-char body must not merge into 45-char remaining space"
        );
    }

    #[test]
    fn body_merge_reports_when_body_short_with_modifiers() {
        let src = "package com.example\n\nclass Test {\n    override suspend fun f(): AuthProvider? =\n        abc()\n}\n";
        let v = fs_check(src);
        let fs: Vec<_> = v
            .iter()
            .filter(|x| x.message.contains("First line of body expression"))
            .collect();
        assert_eq!(fs.len(), 1);
        assert_eq!(fs[0].line, 4);
        assert_eq!(fs[0].col, 46);
    }
}

#[cfg(test)]
mod dbg_m {
    use crate::config::CodeStyle;
    use crate::parser::KotlinParser;
    use crate::rules::phase3b_rules::FunctionSignatureSpacing;
    use crate::rules::Rule;

    #[test]
    fn dump_m117() {
        let src = std::fs::read_to_string("/tmp/verify/i5/m117.kt").unwrap();
        let tree = KotlinParser::new().parse(&src);
        let rule = FunctionSignatureSpacing::new(120, CodeStyle::AndroidStudio);
        let v = rule.check(&tree, &src);
        eprintln!(
            "m117 violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }
}
