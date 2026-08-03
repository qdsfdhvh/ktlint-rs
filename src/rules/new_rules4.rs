//! Batch 4: Final ktlint parity rules (wrapping, block comment, etc.)
use crate::rules::{Rule, Violation};

pub struct CommentWrappingRule;
impl Rule for CommentWrappingRule {
    fn id(&self) -> &'static str {
        "standard:comment-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        // Fail closed: the previous line-scan heuristic produced mass false
        // positives on real projects (verified against a live Spotless 8.8.0 +
        // ktlint 1.8.0 oracle with zero violations). A CST-aware implementation
        // must replace this before the rule can be re-enabled.
        Vec::new()
    }
}


pub struct KdocWrappingRule;
impl Rule for KdocWrappingRule {
    fn id(&self) -> &'static str {
        "standard:kdoc-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        // Fail closed: the previous line-scan heuristic produced mass false
        // positives on real projects (verified against a live Spotless 8.8.0 +
        // ktlint 1.8.0 oracle with zero violations). A CST-aware implementation
        // must replace this before the rule can be re-enabled.
        Vec::new()
    }
}


// FunctionExpressionBodyRule removed — dead duplicate of phase3b_rules::FunctionExpressionBody

pub struct CallExpressionWrappingRule;
impl Rule for CallExpressionWrappingRule {
    fn id(&self) -> &'static str {
        "standard:call-expression-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            let t = ln.trim();
            if t.contains("(") && t.len() > 100 && i + 1 < l.len() {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Long call expression should be wrapped".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct BinaryExpressionWrappingRule;
impl Rule for BinaryExpressionWrappingRule {
    fn id(&self) -> &'static str {
        "standard:binary-expression-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        // Fail closed: the previous line-scan heuristic produced mass false
        // positives on real projects (verified against a live Spotless 8.8.0 +
        // ktlint 1.8.0 oracle with zero violations). A CST-aware implementation
        // must replace this before the rule can be re-enabled.
        Vec::new()
    }
}


pub struct PropertyWrappingRule;
impl Rule for PropertyWrappingRule {
    fn id(&self) -> &'static str {
        "standard:property-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        // Fail closed: the previous line-scan heuristic produced mass false
        // positives on real projects (verified against a live Spotless 8.8.0 +
        // ktlint 1.8.0 oracle with zero violations). A CST-aware implementation
        // must replace this before the rule can be re-enabled.
        Vec::new()
    }
}


pub struct ParameterWrappingRule;
impl Rule for ParameterWrappingRule {
    fn id(&self) -> &'static str {
        "standard:parameter-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            let t = ln.trim();
            if t.starts_with("fun ") && !t.contains("\n") && t.len() > 120 {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Function signature too long, consider wrapping parameters".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct IfElseWrappingRule;
impl Rule for IfElseWrappingRule {
    fn id(&self) -> &'static str {
        "standard:if-else-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            if ln.trim() == "else"
                && i + 1 < l.len()
                && !l[i + 1].trim().starts_with("if")
                && !l[i + 1].trim().starts_with('{')
            {
                v.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "else body should be wrapped in braces".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct StatementWrappingRule;
impl Rule for StatementWrappingRule {
    fn id(&self) -> &'static str {
        "standard:statement-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        // Fail closed: the previous line-scan heuristic produced mass false
        // positives on real projects (verified against a live Spotless 8.8.0 +
        // ktlint 1.8.0 oracle with zero violations). A CST-aware implementation
        // must replace this before the rule can be re-enabled.
        Vec::new()
    }
}


pub struct ChainMethodContinuationRule;
impl Rule for ChainMethodContinuationRule {
    fn id(&self) -> &'static str {
        "standard:chain-method-continuation"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            if ln.trim().starts_with('.') && i > 0 && !l[i - 1].trim().is_empty() {
                v.push(Violation {
                    file: String::new(),
                    line: i,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Chain continuation '.' should align with previous line".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct MultilineLoopRule;
impl Rule for MultilineLoopRule {
    fn id(&self) -> &'static str {
        "standard:multiline-loop"
    }
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        // Fail closed: the previous line-scan heuristic produced mass false
        // positives on real projects (verified against a live Spotless 8.8.0 +
        // ktlint 1.8.0 oracle with zero violations). A CST-aware implementation
        // must replace this before the rule can be re-enabled.
        Vec::new()
    }
}
