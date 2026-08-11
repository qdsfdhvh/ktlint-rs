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
        if node.kind() == "when_expression" {
            check_when(&node, bytes, violations);
        }
        for i in (0..node.child_count()).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
}

/// Oracle: "Body of when entry should be surrounded by braces if any when
/// entry body is surrounded by braces or has a multiline body". A `when`
/// mixes braced and unbraced entries — or holds any multiline entry — then
/// every unbraced entry is reported at its body's first token.
fn check_when(when: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    let entries: Vec<tree_sitter::Node> = when
        .children(&mut when.walk())
        .filter(|c| c.kind() == "when_entry")
        .collect();
    if entries.len() < 2 {
        return;
    }
    let has_braced_or_multiline = entries.iter().any(|e| {
        let mut w = e.walk();
        let children: Vec<tree_sitter::Node> = e.children(&mut w).collect();
        let braced = children.iter().any(|c| c.kind() == "{");
        let multiline = e.start_position().row != e.end_position().row;
        braced || multiline
    });
    if !has_braced_or_multiline {
        return;
    }
    for entry in &entries {
        let mut w = entry.walk();
        let children: Vec<tree_sitter::Node> = entry.children(&mut w).collect();
        // Braced entry: control_structure_body whose content opens with `{`
        // (a bare expression body like `else -> 0` is wrapped too).
        let braced = children.iter().any(|c| {
            if c.kind() != "control_structure_body" {
                return false;
            }
            c.children(&mut c.walk()).any(|cc| cc.kind() == "{")
        });
        if braced {
            continue;
        }
        let mut after_arrow = false;
        for child in children {
            if child.kind() == "->" {
                after_arrow = true;
                continue;
            }
            if after_arrow && child.kind() != "{" && child.kind() != "}" {
                let pos = child.start_position();
                violations.push(Violation {
                    file: String::new(),
                    line: pos.row + 1,
                    col: pos.column + 1,
                    rule_id: "standard:when-entry-bracing".into(),
                    message:
                        "Body of when entry should be surrounded by braces if any when entry body is surrounded by braces or has a multiline body"
                            .into(),
                    auto_fixable: true,
                });
                break;
            }
        }
    }
}
