//! Batch 2: ktlint parity rules (wrapping, declaration, spacing, comment)
use crate::rules::{Rule, Violation};

pub struct FunctionLiteralRule;
impl Rule for FunctionLiteralRule {
    fn id(&self) -> &'static str {
        "standard:function-literal"
    }
    /// Mirrors ktlint 1.8 FunctionLiteralRule (report half):
    /// - `{ a\n    -> … }` — a single parameter and its arrow must stay on
    ///   one line ("No newline expected after parameter").
    /// - `{\n    a, b -> … }` — parameters must stay on the `{` line
    ///   ("No newline expected before parameter").
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "lambda_literal" {
                // Only lambdas with an explicit parameter list are relevant —
                // `buildList { … }` (no parameters) is fine.
                let lp_node = (0..node.child_count()).find_map(|i| {
                    let c = node.child(i)?;
                    (c.kind() == "lambda_parameters").then_some(c)
                });
                let has_params = lp_node.is_some();
                // Issue #260: a lambda whose parameter list spans multiple
                // lines (one parameter per line, Android Studio's multiline
                // lambda format — `{ \n    a,\n    b,\n    -> …`) is legal;
                // ktlint 1.8 reports only when the *single-line* parameter
                // list is separated from `{` or `->` by a newline. Measured
                // oracle matrix: L1 `{ x: Int,\n ->` reports after; L3
                // `{\n x: Int, y: Int ->` reports before; Repro #260's
                // multiline-params shape stays clean.
                let lp_single_line =
                    lp_node.is_some_and(|n| n.start_position().row == n.end_position().row);
                let text = &source[node.start_byte()..node.end_byte()];
                let lbrace = text.find('{');
                let arrow = text.find("->");

                if has_params {
                    if let (Some(lbrace), Some(arrow)) = (lbrace, arrow) {
                        // `{` directly followed by a newline before the parameter
                        // list — a single-line parameter list must stay on the
                        // `{` line.
                        let after_lbrace = &text[lbrace + 1..];
                        let after_ws =
                            after_lbrace.trim_start_matches(|c: char| c == ' ' || c == '\t');
                        if after_ws.starts_with('\n') && lp_single_line {
                            let pos = node.start_byte() + lbrace + 1;
                            let line = source[..pos].bytes().filter(|&b| b == b'\n').count() + 1;
                            let line_start = source[..pos].rfind('\n').map_or(0, |i| i + 1);
                            violations.push(Violation {
                                file: String::new(),
                                line,
                                col: pos - line_start + 1,
                                rule_id: self.id().into(),
                                message: "No newline expected before parameter".into(),
                                auto_fixable: true,
                            });
                        }
                        // A newline directly before `->` — parameter(s) and
                        // arrow must stay together (issue #204: multi-param
                        // lambdas too), for a single-line parameter list.
                        // Only the gap between the parameter list's end and
                        // the arrow counts — the newline right after `{`
                        // (L3 shape) is the "before parameter" case, and the
                        // oracle does not double-report it as "after".
                        let lp_rel_end = lp_node
                            .map(|n| n.end_byte() - node.start_byte())
                            .unwrap_or(lbrace + 1);
                        // tree-sitter-kotlin may include the `->` inside the
                        // lambda_parameters span (some shapes) — clamp so the
                        // gap slice can never invert (kataris corpus panic).
                        let gap = &text[lp_rel_end.min(text.len()).min(arrow)..arrow];
                        if gap.contains('\n') && lp_single_line {
                            // ktlint reports at the end of the parameter list
                            // (oracle: `{ first: Int, second: Int\n ->` -> 3:72).
                            let end_pos = node
                                .children(&mut node.walk())
                                .find(|c| c.kind() == "lambda_parameters")
                                .map(|lp| lp.end_position())
                                .unwrap_or_else(|| node.start_position());
                            violations.push(Violation {
                                file: String::new(),
                                line: end_pos.row + 1,
                                col: end_pos.column + 1,
                                rule_id: self.id().into(),
                                message: "No newline expected after parameter".into(),
                                auto_fixable: true,
                            });
                        }
                    }
                }
            }
            for i in (0..node.child_count()).rev() {
                if let Some(child) = node.child(i) {
                    stack.push(child);
                }
            }
        }
        violations
    }
}

pub struct NoUnitReturnRule;
impl Rule for NoUnitReturnRule {
    fn id(&self) -> &'static str {
        "standard:no-unit-return"
    }
    /// Mirrors ktlint 1.8 NoUnitReturnRule: `fun foo(): Unit { … }` — an
    /// explicit `Unit` return type on a function with a block body is
    /// unnecessary. Expression bodies (`= expr`) keep it.
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "function_declaration" {
                let mut w = node.walk();
                let kids: Vec<tree_sitter::Node> = node.children(&mut w).collect();
                // Find the return type (`user_type` with text `Unit`) and the
                // function body (brace body only).
                let unit_idx = kids.iter().position(|k| {
                    matches!(k.kind(), "user_type" | "not_nullable_type")
                        && source[k.start_byte()..k.end_byte()].trim() == "Unit"
                });
                let body_is_block = kids
                    .iter()
                    .find(|k| k.kind() == "function_body")
                    .is_some_and(|b| {
                        source[b.start_byte()..b.end_byte()]
                            .trim_start()
                            .starts_with('{')
                    });
                if let Some(idx) = unit_idx {
                    if body_is_block {
                        let unit = kids[idx];
                        let line = source[..unit.start_byte()]
                            .bytes()
                            .filter(|&b| b == b'\n')
                            .count()
                            + 1;
                        let line_start =
                            source[..unit.start_byte()].rfind('\n').map_or(0, |i| i + 1);
                        violations.push(Violation {
                            file: String::new(),
                            line,
                            col: unit.start_byte() - line_start + 1,
                            rule_id: self.id().into(),
                            message: "Unnecessary \"Unit\" return type".into(),
                            auto_fixable: true,
                        });
                    }
                }
            }
            for i in (0..node.child_count()).rev() {
                if let Some(child) = node.child(i) {
                    stack.push(child);
                }
            }
        }
        violations
    }
}

pub struct NoSingleLineBlockCommentRule;
impl Rule for NoSingleLineBlockCommentRule {
    fn id(&self) -> &'static str {
        "standard:no-single-line-block-comment"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            let trimmed = l.trim();
            // KDoc (`/** ... */`) is exempt — the rule targets plain block
            // comments used where a line comment would do.
            if trimmed.starts_with("/*") && !trimmed.starts_with("/**") && trimmed.ends_with("*/") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Use // for single-line comments instead of /* */".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct SpacingAroundUnaryOperatorRule;
impl Rule for SpacingAroundUnaryOperatorRule {
    fn id(&self) -> &'static str {
        "standard:unary-op-spacing"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.contains("! ") && !l.contains("!!") && !l.contains("\"") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "No space after unary \"!\"".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct FunKeywordSpacingRule;
impl Rule for FunKeywordSpacingRule {
    fn id(&self) -> &'static str {
        "standard:fun-keyword-spacing"
    }

    fn auto_fixable(&self) -> bool {
        true
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            let t = l.trim();
            if let Some(pos) = t.find("fun") {
                let after_fun = &t[pos + 3..];
                if after_fun.starts_with("  ") {
                    // Double space or space-before-non-paren
                    let col = l.find("fun").unwrap_or(0) + 4;
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col,
                        rule_id: self.id().into(),
                        message: "Single space expected after the fun keyword".into(),
                        auto_fixable: true,
                    });
                }
            }
        }
        v
    }
}

pub struct MixedConditionOperatorsRule;
impl Rule for MixedConditionOperatorsRule {
    fn id(&self) -> &'static str {
        "standard:mixed-condition-operators"
    }
    /// Mirrors ktlint 1.8 MixedConditionOperatorsRule: a logical expression
    /// mixing `&&` and `||` at the same nesting level is hard to read. Report
    /// the outermost logical expression once.
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if matches!(
                node.kind(),
                "disjunction_expression" | "conjunction_expression"
            ) {
                // Only the outermost logical expression is reported.
                let parent_is_logical = node.parent().is_some_and(|p| {
                    matches!(
                        p.kind(),
                        "disjunction_expression" | "conjunction_expression"
                    )
                });
                if !parent_is_logical {
                    // Check the same nesting level for the other operator —
                    // parenthesized subexpressions (`a && (b || c)`) are already
                    // clarified and are skipped (ktlint treats them as a
                    // different level).
                    let mut has_and = false;
                    let mut has_or = false;
                    let mut sub = vec![node];
                    while let Some(n) = sub.pop() {
                        if n.kind() == "conjunction_expression" {
                            has_and = true;
                        } else if n.kind() == "disjunction_expression" {
                            has_or = true;
                        }
                        // Do not descend into parentheses or lambdas — both are
                        // separate nesting levels for ktlint.
                        if n.kind() == "parenthesized_expression" || n.kind() == "lambda_literal" {
                            continue;
                        }
                        let mut w = n.walk();
                        for c in n.children(&mut w) {
                            sub.push(c);
                        }
                    }
                    if has_and && has_or {
                        let line = source[..node.start_byte()]
                            .bytes()
                            .filter(|&b| b == b'\n')
                            .count()
                            + 1;
                        let line_start =
                            source[..node.start_byte()].rfind('\n').map_or(0, |i| i + 1);
                        violations.push(Violation {
                            file: String::new(),
                            line,
                            col: node.start_byte() - line_start + 1,
                            rule_id: self.id().into(),
                            message: "A condition with mixed usage of '&&' and '||' is hard to read. Use parenthesis to clarify the (sub)condition.".into(),
                            auto_fixable: false,
                        });
                    }
                }
            }
            for i in (0..node.child_count()).rev() {
                if let Some(child) = node.child(i) {
                    stack.push(child);
                }
            }
        }
        violations
    }
}

#[cfg(test)]
mod function_literal_tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(source);
        FunctionLiteralRule.check(&tree, source)
    }

    #[test]
    fn single_param_arrow_on_next_line_reports_after() {
        // oracle L1: `{ x: Int,\n ->` reports "No newline expected after parameter"
        let src = "package com.example\n\npublic class Test {\n    public fun a() {\n        val f = { x: Int,\n            ->\n            use(x)\n        }\n        use(f)\n    }\n}\n";
        let v = check(src);
        assert!(
            v.iter().any(|x| x.message.contains("after parameter")),
            "violations: {:?}",
            v.iter().map(|x| &x.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn multiline_param_list_is_clean() {
        // Issue #260 repro: one parameter per line + trailing comma + arrow on
        // its own line is legal Android Studio formatting — oracle is clean.
        let src = "package com.example\n\npublic class FunctionLiteral {\n    public fun bind() {\n        val latest = rememberUpdatedState<(String, String) -> Unit> {\n                exampleEmail,\n                examplePassword,\n            ->\n            handle(exampleEmail, examplePassword)\n        }\n        use(latest)\n    }\n}\n";
        assert!(check(src).is_empty());
    }

    #[test]
    fn lbrace_newline_reports_before_but_not_after() {
        // oracle L3: `{\n x: Int, y: Int ->` reports only "before parameter"
        let src = "package com.example\n\npublic class Test {\n    public fun a() {\n        val f = {\n            x: Int, y: Int -> use(x, y)\n        }\n        use(f)\n    }\n}\n";
        let v = check(src);
        assert!(
            v.iter().any(|x| x.message.contains("before parameter")),
            "must report before: {:?}",
            v.iter().map(|x| &x.message).collect::<Vec<_>>()
        );
        assert!(
            !v.iter().any(|x| x.message.contains("after parameter")),
            "must NOT report after (params and arrow share the line)"
        );
    }
}
