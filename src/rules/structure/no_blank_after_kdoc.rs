//! standard:no-empty-line-after-kdoc — no blank line between KDoc and declaration.
use crate::rules::{Rule, Violation};
pub struct NoBlankAfterKdoc;
impl Rule for NoBlankAfterKdoc {
    fn id(&self) -> &'static str {
        "standard:no-empty-line-after-kdoc"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            if ln.trim().ends_with("*/") && i + 1 < l.len() && l[i + 1].trim().is_empty() {
                v.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Unexpected blank line after KDoc comment".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        NoBlankAfterKdoc.check(&p.parse(s), s)
    }
    #[test]
    fn ok() {
        assert!(c("/** doc */\nclass Foo\n").is_empty());
    }
    #[test]
    fn bad() {
        assert!(!c("/** doc */\n\nclass Foo\n").is_empty());
    }
}
