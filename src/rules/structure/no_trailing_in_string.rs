//! standard:no-trailing-spaces-in-string-template — no trailing spaces in string templates.
use crate::rules::{Rule, Violation};
pub struct NoTrailingSpacesInString;
impl Rule for NoTrailingSpacesInString {
    fn id(&self) -> &'static str {
        "standard:no-trailing-spaces-in-string-template"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, ln) in s.lines().enumerate() {
            let t = ln.trim();
            if (t.contains("$\"") || t.contains("\"${")) && ln.ends_with(' ') {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: ln.len() + 1,
                    rule_id: self.id().into(),
                    message: "Trailing space in string template".into(),
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
        NoTrailingSpacesInString.check(&p.parse(s), s)
    }
    #[test]
    fn ok() {
        assert!(c("val x = \"hello\"\n").is_empty());
    }
}
