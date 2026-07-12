//! Unimplemented ktlint-standard rules that exist as test files but not in ktlint-rs.
//! This file tracks our gap to full ktlint parity.
//!
//! Gap: 59 unmapped test files → ~40 missing rules
//! Current: 62 rules implemented
//! Target: 100+ rules for full parity

use crate::rules::{Rule, Violation};

// ── Basic spacing rules (low effort, high impact) ──

pub struct SpacingAroundDot;
impl Rule for SpacingAroundDot {
    fn id(&self) -> &'static str {
        "standard:spacing-around-dot"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.trim().contains(" .") || l.contains(". ") {
                if !l.contains("\"") && !l.contains("..") {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Unexpected space around \".\"".into(),
                        auto_fixable: true,
                    });
                }
            }
        }
        v
    }
}

pub struct SpacingAroundSquareBrackets;
impl Rule for SpacingAroundSquareBrackets {
    fn id(&self) -> &'static str {
        "standard:spacing-around-square-brackets"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            let t = l.trim();
            if t.contains("[ ") || t.contains(" ]") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Unexpected space inside square brackets".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct NoBlankLinesInChainedMethodCalls;
impl Rule for NoBlankLinesInChainedMethodCalls {
    fn id(&self) -> &'static str {
        "standard:no-blank-lines-in-chained-method-calls"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        let mut in_chain = false;
        for (i, ln) in l.iter().enumerate() {
            if ln.trim().starts_with('.') || ln.trim().contains("?.") {
                in_chain = true;
            }
            if in_chain && ln.trim().is_empty() {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Unexpected blank line in chained method call".into(),
                    auto_fixable: true,
                });
            }
            if !ln.trim().starts_with('.') && !ln.trim().is_empty() && in_chain {
                in_chain = false;
            }
        }
        v
    }
}

pub struct NoLineBreakAfterElse;
impl Rule for NoLineBreakAfterElse {
    fn id(&self) -> &'static str {
        "standard:no-line-break-after-else"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            if ln.trim() == "else" && i + 1 < l.len() && l[i + 1].trim().is_empty() {
                v.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Unexpected blank line after else".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct NoLineBreakBeforeAssignment;
impl Rule for NoLineBreakBeforeAssignment {
    fn id(&self) -> &'static str {
        "standard:no-line-break-before-assignment"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            if ln.trim().starts_with('=') && i > 0 && !l[i - 1].trim().is_empty() {
                v.push(Violation {
                    file: String::new(),
                    line: i,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Line break before \"=\" should be avoided".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct NoConsecutiveComments;
impl Rule for NoConsecutiveComments {
    fn id(&self) -> &'static str {
        "standard:no-consecutive-comments"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for i in 0..l.len().saturating_sub(1) {
            if l[i].trim().starts_with("//") && l[i + 1].trim().starts_with("//") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Consecutive comments should be combined".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct NullableTypeSpacing;
impl Rule for NullableTypeSpacing {
    fn id(&self) -> &'static str {
        "standard:nullable-type-spacing"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.contains(" ?") && l.contains('?') && !l.contains("\"") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "No space before \"?\" in nullable type".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}
