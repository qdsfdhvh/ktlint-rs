//! standard:no-empty-file-body — flag unnecessarily empty function/class bodies on multi-line.
use crate::rules::{Rule, Violation};

pub struct NoEmptyFileBody;

impl Rule for NoEmptyFileBody {
    fn id(&self) -> &'static str { "standard:no-empty-file-body" }
    fn auto_fixable(&self) -> bool { false }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t == "{}" || t == "{ }" { continue; }
            if t.ends_with('{') && i+1 < lines.len() && lines[i+1].trim() == "}" {
                violations.push(Violation {
                    file: String::new(), line: i+1, col: 1,
                    rule_id: self.id().to_string(),
                    message: "Empty body should be on single line `{}`".to_string(),
                    auto_fixable: true,
                });
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*; use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> { let mut p=KotlinParser::new(); NoEmptyFileBody.check(&p.parse(s), s) }
    #[test] fn single_line() { assert!(check("fun foo() {}\n").is_empty()); }
    #[test] fn multiline_empty() { let v=check("fun foo() {\n}\n"); assert!(!v.is_empty()); }
}
