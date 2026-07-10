//! standard:ij_kotlin_allow_trailing_comma — IntelliJ trailing comma compatibility.
use crate::rules::{Rule, Violation};
pub struct IJTrailingComma;
impl Rule for IJTrailingComma {
    fn id(&self) -> &'static str {
        "standard:ij_kotlin_allow_trailing_comma"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, ln) in s.lines().enumerate() {
            let t = ln.trim();
            if (t.starts_with("fun ") || t.starts_with("class "))
                && t.contains(',')
                && t.contains(')')
            {
                // Check for trailing comma before )
                if let Some(p) = t.rfind(')') {
                    if p > 0 && t.as_bytes()[p - 1] == b',' {
                        v.push(Violation{file:String::new(),line:i+1,col:p+1,rule_id:self.id().into(),
                            message:"Trailing comma call-site (configurable via ij_kotlin_allow_trailing_comma)".into(),
                            auto_fixable:true});
                    }
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
        IJTrailingComma.check(&p.parse(s), s)
    }
    #[test]
    fn ok() {
        assert!(c("fun f(a: Int, b: String)\n").is_empty());
    }
    #[test]
    fn trailing() {
        assert!(!c("fun f(a: Int, b: String,)\n").is_empty());
    }
}
