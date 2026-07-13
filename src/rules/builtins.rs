//! Built-in simple rules — NoTrailingSpaces, FinalNewline, NoConsecutiveBlankLines, NoWildcardImports.
//! Extracted from rules/mod.rs (Phase 8).

use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

pub struct NoTrailingSpaces;
impl Rule for NoTrailingSpaces {
    fn id(&self) -> &'static str { "standard:no-trailing-spaces" }
    fn check(&self, _: &Tree, s: &str) -> Vec<Violation> {
        s.lines()
            .enumerate()
            .filter(|(_, l)| l.ends_with(' ') || l.ends_with('\t'))
            .map(|(i, _)| Violation {
                file: String::new(), line: i + 1, col: 0,
                rule_id: self.id().into(),
                message: "Trailing space(s)".into(),
                auto_fixable: true,
            })
            .collect()
    }
}

pub struct FinalNewline;
impl Rule for FinalNewline {
    fn id(&self) -> &'static str { "standard:final-newline" }
    fn check(&self, _: &Tree, s: &str) -> Vec<Violation> {
        if s.is_empty() || s.ends_with('\n') { vec![] }
        else { vec![Violation {
            file: String::new(), line: s.lines().count(), col: 0,
            rule_id: self.id().into(),
            message: "File must end with a newline".into(),
            auto_fixable: true,
        }]}
    }
}

pub struct NoConsecutiveBlankLines;
impl Rule for NoConsecutiveBlankLines {
    fn id(&self) -> &'static str { "standard:no-consecutive-blank-lines" }
    fn check(&self, _: &Tree, s: &str) -> Vec<Violation> {
        let mut v = vec![];
        let mut b = 0;
        for (i, l) in s.lines().enumerate() {
            if l.trim().is_empty() { b += 1; if b > 1 {
                v.push(Violation {
                    file: String::new(), line: i + 1, col: 0,
                    rule_id: self.id().into(),
                    message: "Needless blank line(s)".into(),
                    auto_fixable: true,
                });
            } } else { b = 0; }
        }
        v
    }
}

pub struct NoWildcardImports;
impl Rule for NoWildcardImports {
    fn id(&self) -> &'static str { "standard:no-wildcard-imports" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _: &Tree, s: &str) -> Vec<Violation> {
        s.lines()
            .enumerate()
            .filter(|(_, l)| { let t = l.trim(); t.starts_with("import ") && t.contains(".*") })
            .map(|(i, _)| Violation {
                file: String::new(), line: i + 1, col: 1,
                rule_id: self.id().into(),
                message: "Wildcard import".into(),
                auto_fixable: false,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    #[test] fn trailing_spaces() {
        let r = NoTrailingSpaces;
        assert!(!r.check(&crate::parser::KotlinParser::new().parse("x "), "x ").is_empty());
        assert!(r.check(&crate::parser::KotlinParser::new().parse("x"), "x").is_empty());
    }

    #[test] fn final_newline() {
        let r = FinalNewline;
        let empty: &Tree = &KotlinParser::new().parse("fun a() {}");
        assert!(!r.check(empty, "fun a() {}").is_empty());
        assert!(r.check(empty, "fun a() {}\n").is_empty());
    }
}
