//! standard:multiline-if-else — closing brace and else/newline consistency.
use crate::rules::{Rule, Violation};

pub struct MultilineIfElse;

impl Rule for MultilineIfElse {
    fn id(&self) -> &'static str { "standard:multiline-if-else" }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("else") && i > 0 {
                let prev_trimmed = lines[i - 1].trim();
                if !prev_trimmed.ends_with('}') || !prev_trimmed.contains("else") {
                    violations.push(Violation {
                        file: String::new(), line: i + 1, col: 1,
                        rule_id: self.id().to_string(), auto_fixable: true,
                        message: "\"else\" should be on same line as preceding \"}\"".into(),
                    });
                }
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*; use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> { let mut p=KotlinParser::new(); MultilineIfElse.check(&p.parse(s), s) }
    #[test] fn else_on_same_line() { assert!(c("if (x) {\n    doA()\n} else {\n    doB()\n}\n").is_empty()); }
    #[test] fn else_on_new_line_bad() { let v=c("if (x) {\n    doA()\n}\nelse {\n    doB()\n}\n"); assert!(!v.is_empty()); }
    #[test] fn else_if() { assert!(c("if(x){\ndoA()\n}else if(y){\ndoB()\n}\n").is_empty()); }
}
