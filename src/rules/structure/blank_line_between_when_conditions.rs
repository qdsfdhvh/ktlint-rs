//! standard:blank-line-between-when-conditions
//!
//! ktlint 1.8: when at least one when-condition is multiline (or a branch is
//! preceded by a comment), a blank line is required between every
//! when-condition. Otherwise branches sit adjacent.
use crate::rules::{Rule, Violation};

pub struct BlankLineBetweenWhenConditions;

impl Rule for BlankLineBetweenWhenConditions {
    fn id(&self) -> &'static str {
        "standard:blank-line-between-when-conditions"
    }
    fn check(&self, tree: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "when_expression" {
                // Direct when-entry children (skip nested whens inside bodies).
                let entries: Vec<tree_sitter::Node> = {
                    let mut w = node.walk();
                    node.children(&mut w)
                        .filter(|c| c.kind() == "when_entry")
                        .collect()
                };
                if entries.len() < 2 {
                    let mut w = node.walk();
                    for c in node.children(&mut w) {
                        stack.push(c);
                    }
                    continue;
                }
                // A condition is multiline when the text before `->` (or the
                // whole entry, when there is no arrow at the top level) spans
                // more than one line.
                let has_multiline = entries.iter().any(|e| {
                    let text = &s[e.start_byte()..e.end_byte()];
                    match text.find("->") {
                        Some(arrow) => {
                            let cond = &text[..arrow];
                            cond.contains('\n')
                        }
                        None => e.start_position().row != e.end_position().row,
                    }
                });
                if has_multiline {
                    for k in 1..entries.len() {
                        let prev = entries[k - 1];
                        let cur = entries[k];
                        // Whitespace between the previous entry's end and this
                        // entry's start must contain a blank line.
                        let gap = &s[prev.end_byte()..cur.start_byte()];
                        let blank_count = gap.matches('\n').count();
                        if blank_count < 2 {
                            let line = s[..cur.start_byte()]
                                .bytes()
                                .filter(|&b| b == b'\n')
                                .count()
                                + 1;
                            v.push(Violation {
                                file: String::new(),
                                line,
                                col: 1,
                                rule_id: self.id().into(),
                                message:
                                    "Add a blank line between all when-conditions in case at least one multiline when-condition is found in the statement"
                                        .into(),
                                auto_fixable: true,
                            });
                        }
                    }
                }
            }
            let mut w = node.walk();
            for c in node.children(&mut w) {
                stack.push(c);
            }
        }
        v
    }
}
