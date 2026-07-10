//! standard:try-catch-finally-wrapping — try/catch/finally brace wrapping.
use crate::rules::{Rule, Violation};

pub struct TryCatchFinallyWrapping;
impl Rule for TryCatchFinallyWrapping {
    fn id(&self) -> &'static str { "standard:try-catch-finally-wrapping" }
    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("catch") || t.starts_with("finally") {
                if i > 0 {
                    let prev = lines[i-1].trim();
                    if !prev.contains("catch") && !prev.contains("finally") {
                        v.push(Violation { file: String::new(), line: i+1, col: 1,
                            rule_id: self.id().to_string(), auto_fixable: true,
                            message: "\"catch\"/\"finally\" should be on same line as \"}\"".into(),
                        });
                    }
                }
            }
        }
        v
    }
}
#[cfg(test)] mod tests { use super::*; use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> { let mut p=KotlinParser::new(); TryCatchFinallyWrapping.check(&p.parse(s), s) }
    #[test] fn ok() { assert!(c("try { a() } catch(e: E) { b() }\n").is_empty()); }
    #[test] fn bad() { assert!(!c("try { a() }\ncatch(e: E) { b() }\n").is_empty()); }
}
