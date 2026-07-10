//! standard:when-expression-line-break — when entries should be consistent.
use crate::rules::{Rule, Violation};
pub struct WhenExpressionLineBreak;
impl Rule for WhenExpressionLineBreak {
    fn id(&self) -> &'static str {
        "standard:when-expression-line-break"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            let t = ln.trim();
            if t.starts_with("when") && t.ends_with('{') {
                let mut entries = 0;
                let mut multiline = 0;
                for j in i + 1..l.len().min(i + 50) {
                    let u = l[j].trim();
                    if u == "}" {
                        break;
                    }
                    if u.contains("->") {
                        entries += 1;
                        if u.contains("->")
                            && (u.ends_with("{")
                                || (u.trim().ends_with("->")
                                    && j + 1 < l.len()
                                    && !l[j + 1].trim().is_empty()
                                    && l[j + 1].trim() != "}"))
                        {
                            multiline += 1;
                        }
                    }
                }
                if entries > 0 && multiline > 0 && multiline < entries {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message:
                            "When entries should consistently use same line or multiline bodies"
                                .into(),
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
        WhenExpressionLineBreak.check(&p.parse(s), s)
    }
    #[test]
    fn consistent() {
        assert!(c("when(x){\n 1->\"a\"\n 2->\"b\"\n}\n").is_empty());
    }
    #[test]
    fn mixed() {
        assert!(!c("when(x){\n 1->\"a\"\n 2->{\n doB()\n }\n}\n").is_empty());
    }
}
