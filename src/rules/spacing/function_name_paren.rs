//! standard:spacing-between-function-name-and-parenthesis — no space before (.
use crate::rules::{Rule, Violation};

pub struct FunctionNameParenSpacing;

impl Rule for FunctionNameParenSpacing {
    fn id(&self) -> &'static str {
        "standard:spacing-between-function-name-and-parenthesis"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            // Pattern: `fun name (params)` — space before ( in function declaration
            if trimmed.starts_with("fun ") {
                // Find the function name then check for space before (
                if let Some(fun_end) = trimmed.find("fun ") {
                    let after_fun = &trimmed[fun_end + 4..];
                    if let Some(name_end) = after_fun.find('(') {
                        let name = after_fun[..name_end].trim();
                        if name.contains(' ') {
                            // Space in the function name — not our concern, other rules handle naming
                        } else if name_end > 0 && after_fun.as_bytes()[name_end - 1] == b' ' {
                            violations.push(Violation {
                                file: String::new(),
                                line: i + 1,
                                col: fun_end + 5 + name_end,
                                rule_id: self.id().to_string(),
                                message: "Unexpected space between function name and \"(\""
                                    .to_string(),
                                auto_fixable: true,
                            });
                        }
                    }
                }
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        let t = p.parse(s);
        FunctionNameParenSpacing.check(&t, s)
    }
    #[test]
    fn normal_fun() {
        assert!(check("fun foo()\n").is_empty());
    }
    #[test]
    fn space_before_paren() {
        let v = check("fun foo ()\n");
        assert!(!v.is_empty());
        assert_eq!(
            v[0].rule_id,
            "standard:spacing-between-function-name-and-parenthesis"
        );
    }
}
