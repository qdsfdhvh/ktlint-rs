//! standard:spacing-between-declarations-with-comments
use crate::rules::{Rule, Violation};

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
