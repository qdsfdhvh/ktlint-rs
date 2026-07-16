//! standard:when-entry-bracing — CST-based check for when entries w/o braces.
use crate::rules::{Rule, Violation};

pub struct WhenEntryBracing;

impl Rule for WhenEntryBracing {
    fn id(&self) -> &'static str {
        "standard:when-entry-bracing"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        walk(tree.root_node(), source.as_bytes(), &mut violations);
        violations
    }
}

fn walk(root: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    let mut stack: Vec<tree_sitter::Node> = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "when_entry" {
            check_entry(&node, bytes, violations);
        }
        for i in (0..node.child_count()).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
}

fn check_entry(entry: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    let entry_line = entry.start_position().row;
    let entry_end_line = entry.end_position().row;
    if entry_end_line <= entry_line {
        return;
    }

    let mut found_arrow = false;
    for i in 0..entry.child_count() {
        let Some(child) = entry.child(i) else {
            continue;
        };
        if child.kind() == "->" {
            found_arrow = true;
            continue;
        }
        if found_arrow && !child.is_extra() {
            if child.start_position().row > entry_line && child.kind() != "{" {
                let s = child.start_byte();
                let e = (s + 5).min(bytes.len());
                if !bytes[s..e].contains(&b'{') {
                    violations.push(Violation {
                        file: String::new(),
                        line: entry_line + 1,
                        col: 1,
                        rule_id: "standard:when-entry-bracing".into(),
                        message: "When entry with multi-line body should use braces".into(),
                        auto_fixable: true,
                    });
                }
            }
            break;
        }
    }
}
