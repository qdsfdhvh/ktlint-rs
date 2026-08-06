//! Batch 2: ktlint parity rules (wrapping, declaration, spacing, comment)
use crate::rules::{Rule, Violation};

pub struct AnnotationRule;
impl Rule for AnnotationRule {
    fn id(&self) -> &'static str {
        "standard:annotation"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        vec![]
    } // disabled — too noisy
}

pub struct FunctionLiteralRule;
impl Rule for FunctionLiteralRule {
    fn id(&self) -> &'static str {
        "standard:function-literal"
    }
    fn check(&self, _tree: &tree_sitter::Tree, _source: &str) -> Vec<Violation> {
        // The real ktlint rule formats lambda parameter lists and arrows. The old
        // line heuristic flagged every `val x: () -> Unit = {}` declaration, so
        // remain fail-closed until the CST implementation is ported and verified.
        Vec::new()
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
            if l.trim().starts_with("/*") && l.trim().ends_with("*/") {
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

pub struct BlankLineBeforeDeclarationRule;
impl Rule for BlankLineBeforeDeclarationRule {
    fn id(&self) -> &'static str {
        "standard:blank-line-before-declaration"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for i in 1..l.len() {
            let t = l[i].trim();
            if t.starts_with("fun ")
                || t.starts_with("class ")
                || t.starts_with("val ")
                || t.starts_with("var ")
            {
                let prev = l[i - 1].trim();
                if !prev.is_empty() && !prev.starts_with("//") && !prev.starts_with("@") {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Blank line required before declaration".into(),
                        auto_fixable: true,
                    });
                }
            }
        }
        v
    }
}

pub struct SpacingAroundAngleBracketsRule;
impl Rule for SpacingAroundAngleBracketsRule {
    fn id(&self) -> &'static str {
        "standard:spacing-around-angle-brackets"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let _bytes = s.as_bytes();
        for (i, l) in s.lines().enumerate() {
            let t = l.trim();
            if t.contains("< ") && !t.contains("<<") && !t.contains("\"") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "No space after \"<\" in type arguments".into(),
                    auto_fixable: true,
                });
            }
            if t.contains(" >") && !t.contains(">>") && !t.contains("->") && !t.contains("\"") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "No space before \">\" in type arguments".into(),
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

pub struct PackageImportSpacingRule;
impl Rule for PackageImportSpacingRule {
    fn id(&self) -> &'static str {
        "standard:package-import-spacing"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        let mut saw_package = false;
        let mut saw_import = false;
        for (i, ln) in l.iter().enumerate() {
            let t = ln.trim();
            if t.starts_with("package ") {
                saw_package = true;
            }
            if t.starts_with("import ") {
                if saw_package && !saw_import {
                    saw_import = true;
                }
            }
            if saw_import
                && t.is_empty()
                && i + 1 < l.len()
                && l[i + 1].trim().starts_with("import ")
            {
                v.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "No blank line between package and imports".into(),
                    auto_fixable: true,
                });
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
