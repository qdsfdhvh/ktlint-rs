//! standard:no-empty-first-line-in-class-body — focused: only class/interface/object.
use crate::rules::{Rule, Violation};
pub struct NoEmptyFirstLineInClassBody;
impl Rule for NoEmptyFirstLineInClassBody {
    fn id(&self) -> &'static str {
        "standard:no-empty-first-line-in-class-body"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            let t = ln.trim();
            // Only class/interface/object, NOT any brace block
            if t.ends_with('{')
                && (t.starts_with("class ")
                    || t.starts_with("interface ")
                    || t.starts_with("object "))
            {
                if i + 1 < l.len() && l[i + 1].trim().is_empty() {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 2,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Unexpected blank line in class body".into(),
                        auto_fixable: true,
                    });
                }
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
        NoEmptyFirstLineInClassBody.check(&p.parse(s), s)
    }
    #[test]
    fn ok() {
        assert!(c("class Foo {\n    val x=1\n}\n").is_empty());
    }
    #[test]
    fn bad() {
        assert!(!c("class Foo {\n\n    val x=1\n}\n").is_empty());
    }
    #[test]
    fn fun_ignored() {
        assert!(c("fun bar() {\n    return 1\n}\n").is_empty());
    }
}
