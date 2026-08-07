//! annotation/wrapping/no-consecutive-comments rules — JVM parity
use crate::rules::{Rule, Violation};

pub struct KtlintNoConsecutiveComments;
impl Rule for KtlintNoConsecutiveComments {
    fn id(&self) -> &'static str {
        "standard:no-consecutive-comments"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for i in 0..l.len().saturating_sub(1) {
            let a = l[i].trim();
            let b = l[i + 1].trim();
            if a.starts_with("//")
                && !a.starts_with("///")
                && !a.starts_with("////")
                && b.starts_with("//")
                && !b.starts_with("///")
            {
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
