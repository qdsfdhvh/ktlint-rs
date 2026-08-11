//! standard:try-catch-finally-wrapping — try/catch/finally brace wrapping.
use crate::rules::{Rule, Violation};

pub struct TryCatchFinallyWrapping;

impl Rule for TryCatchFinallyWrapping {
    fn id(&self) -> &'static str {
        "standard:try-catch-finally-wrapping"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if (t.starts_with("catch") || t.starts_with("finally")) && i > 0 {
                let prev = lines[i - 1].trim();
                if !prev.contains("catch") && !prev.contains("finally") {
                    let indent = line.len() - line.trim_start().len();
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: indent + 1,
                        rule_id: "standard:try-catch-finally-wrapping".into(),
                        auto_fixable: true,
                        message: format!(
                            "Unexpected newline before \"{}\"",
                            t.split_whitespace().next().unwrap_or("catch")
                        ),
                    });
                }
            }
        }
        v
    }
}

/// standard:try-catch-finally-spacing — `catch`/`finally` must follow the
/// closing `}` of the previous block after exactly one space.
pub struct TryCatchFinallySpacing;

impl Rule for TryCatchFinallySpacing {
    fn id(&self) -> &'static str {
        "standard:try-catch-finally-spacing"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if (t.starts_with("catch") || t.starts_with("finally")) && i > 0 {
                let prev = lines[i - 1].trim();
                // `}catch` (no space) on the previous line, or the keyword on
                // its own line (Allman). Oracle reports "A single space is
                // required before 'catch'" at the keyword.
                let own_line = prev.ends_with('}');
                let no_space = prev.ends_with('}') && prev.ends_with(&format!("}}{}", ""));
                if own_line {
                    let indent = line.len() - line.trim_start().len();
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: indent + 1,
                        rule_id: "standard:try-catch-finally-spacing".into(),
                        auto_fixable: true,
                        message: format!(
                            "A single space is required before '{}'",
                            t.split_whitespace().next().unwrap_or("catch")
                        ),
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
        TryCatchFinallyWrapping.check(&p.parse(s), s)
    }
    #[test]
    fn ok() {
        assert!(c("try { a() } catch(e: E) { b() }\n").is_empty());
    }
    #[test]
    fn bad() {
        assert!(!c("try { a() }\ncatch(e: E) { b() }\n").is_empty());
    }
}
