//! standard:blank-line-between-when-conditions
//!
//! ktlint separates a when-branch whose body is a block (`{ ... }`) from the
//! previous branch with a blank line. Branches with simple-expression bodies
//! may sit adjacent. Mirrors ktlint 1.8 behavior verified against the live
//! Spotless oracle.
use crate::rules::{Rule, Violation};

pub struct BlankLineBetweenWhenConditions;

impl Rule for BlankLineBetweenWhenConditions {
    fn id(&self) -> &'static str {
        "standard:blank-line-between-when-conditions"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        let mut in_when = false;
        let mut prev_is_block = false;
        for i in 0..l.len() {
            let t = l[i].trim();
            if !in_when {
                // `when` can sit mid-expression (`val x = when(x) {`); detect by
                // word boundary to avoid `somewhen` false positives.
                let has_when = t
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|w| w == "when");
                if has_when && t.contains('{') && !t.contains('}') {
                    in_when = true;
                    prev_is_block = false;
                }
                continue;
            }
            if t == "}" {
                in_when = false;
                continue;
            }
            if t.contains("->") {
                let this_is_block = t.ends_with('{');
                // A block-body branch is separated from any previous branch;
                // the first branch after `when {` needs no separator.
                let has_prev_branch = i > 1 && (prev_is_block || l[i - 1].trim().contains("->"));
                if this_is_block && has_prev_branch && !l[i - 1].trim().is_empty() {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Consider blank line between when conditions".into(),
                        auto_fixable: true,
                    });
                }
                prev_is_block = this_is_block;
            } else if !t.is_empty() {
                // Inside a branch block body; prev branch was a block.
            }
        }
        v
    }
}
