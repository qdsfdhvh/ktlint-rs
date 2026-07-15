//! standard:blank-line-between-when-conditions
use crate::rules::{Rule, Violation};

pub struct BlankLineBetweenWhenConditions;

impl Rule for BlankLineBetweenWhenConditions {
    fn id(&self) -> &'static str { "standard:blank-line-between-when-conditions" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        let mut in_when = false;
        for i in 0..l.len() {
            let t = l[i].trim();
            if t.starts_with("when ") || t.starts_with("when(") { in_when = true; }
            if in_when && t == "}" { in_when = false; }
            if in_when && t.contains("->") && i + 1 < l.len() {
                let next = l[i + 1].trim();
                if !next.is_empty() && next.contains("->") {
                    v.push(Violation {
                        file: String::new(), line: i + 2, col: 1,
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
