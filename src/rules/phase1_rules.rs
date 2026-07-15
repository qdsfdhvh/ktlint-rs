//! Phase 1 batch: when-entry-bracing, blank-line-before-declaration, wrapping, trailing-comma
use crate::rules::{Rule, Violation};

pub struct WhenEntryBracing;
impl Rule for WhenEntryBracing {
    fn id(&self) -> &'static str { "standard:when-entry-bracing" }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        walk(tree.root_node(), source.as_bytes(), &mut violations);
        violations
    }
}

fn walk(node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    if node.kind() == "when_entry" {
        check_entry(&node, bytes, violations);
    }
    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) { walk(c, bytes, violations); }
    }
}

fn check_entry(entry: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
    let entry_line = entry.start_position().row;
    let entry_end_line = entry.end_position().row;
    if entry_end_line <= entry_line { return; }

    let mut found_arrow = false;
    for i in 0..entry.child_count() {
        let Some(child) = entry.child(i) else { continue };
        if child.kind() == "->" { found_arrow = true; continue; }
        if found_arrow && !child.is_extra() {
            if child.start_position().row > entry_line
                && child.kind() != "{"
            {
                let s = child.start_byte();
                let e = (s + 5).min(bytes.len());
                if !bytes[s..e].contains(&b'{') {
                    violations.push(Violation {
                        file: String::new(), line: entry_line + 1, col: 1,
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

pub struct BlankLineBetweenWhenConditions;
impl Rule for BlankLineBetweenWhenConditions {
    fn id(&self) -> &'static str {
        "standard:blank-line-between-when-conditions"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        let mut in_when = false;
        for i in 0..l.len() {
            let t = l[i].trim();
            if t.starts_with("when ") || t.starts_with("when(") {
                in_when = true;
            }
            if in_when && t == "}" {
                in_when = false;
            }
            if in_when && t.contains("->") && i + 1 < l.len() {
                let next = l[i + 1].trim();
                if !next.is_empty() && next.contains("->") {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 2,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Consider blank line between when conditions".into(),
                        auto_fixable: true,
                    });
                }
            }
        }
        v
    }
}

pub struct TrailingCommaOnCallSite;
impl Rule for TrailingCommaOnCallSite {
    fn id(&self) -> &'static str {
        "standard:trailing-comma-on-call-site"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            let t = l.trim();
            if t.contains("(")
                && t.contains(")")
                && !t.starts_with("fun ")
                && !t.starts_with("class ")
            {
                if let Some(rp) = t.rfind(')') {
                    if rp > 1 && t.as_bytes()[rp - 1] == b',' {
                        v.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: rp + 1,
                            rule_id: self.id().into(),
                            message: "Trailing comma on call site".into(),
                            auto_fixable: true,
                        });
                    }
                }
            }
        }
        v
    }
}

pub struct SpacingBetweenDeclarationsWithComments;
impl Rule for SpacingBetweenDeclarationsWithComments {
    fn id(&self) -> &'static str {
        "standard:spacing-between-declarations-with-comments"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for i in 0..l.len() {
            let t = l[i].trim();
            if t.starts_with("//") && i + 1 < l.len() {
                let next = l[i + 1].trim();
                if (next.starts_with("fun ") || next.starts_with("class "))
                    && i > 0
                    && !l[i - 1].trim().is_empty()
                {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Spacing between declarations with comments".into(),
                        auto_fixable: true,
                    });
                }
            }
        }
        v
    }
}
