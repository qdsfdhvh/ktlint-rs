//! Phase 3.3 semantics: no-empty-file, indent, max-line, kdoc, function-signature
use crate::rules::{Rule, Violation};

pub struct FunctionSignatureSpacing {
    max_length: usize,
}

impl FunctionSignatureSpacing {
    pub fn new(max_length: usize) -> Self {
        let max_length = if max_length == 0 { 120 } else { max_length };
        Self { max_length }
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
                        // chain) must stay on its own line — ktlint's
                        // multiline-expression-wrapping demands it, so
                        // "fits on same line" never applies (issue #160).
                        if expr.start_position().row != expr.end_position().row {
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
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            let t = ln.trim();
            if t.starts_with("fun ") && t.contains('(') && !t.contains(')') {
                let after_open = t.split_once('(').map_or("", |(_, rest)| rest).trim();
                if !after_open.is_empty() {
                    let next = l.get(i + 1).copied().unwrap_or("");
                    v.push(Violation {
                        file: String::new(),
                        line: i + 2,
                        col: next.len() - next.trim_start().len() + 1,
                        rule_id: self.id().into(),
                        message: "Single whitespace expected before parameter".into(),
                        auto_fixable: true,
                    });
                }
            }
        }
        self.check_body_merge(tree, s, &mut v);
        v
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
        FunctionSignatureSpacing::new(120).check(&tree, source)
    }

    #[test]
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
