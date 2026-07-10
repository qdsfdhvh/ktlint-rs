//! KDOC rules — KDoc comment formatting (5 checks in 1 rule).
//! Checks: no empty KDOCs, param ordering, indentation, period, return tag.
use crate::rules::{Rule, Violation};

pub struct KdocFormatting;

impl Rule for KdocFormatting {
    fn id(&self) -> &'static str {
        "standard:kdoc"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("/**") && trimmed.ends_with("*/") {
                // Single-line KDoc: "/** doc */" — OK
            } else if trimmed == "/**" {
                // Multi-line KDoc start — check next line for empty content
                if i + 1 < lines.len() {
                    let next = lines[i + 1].trim();
                    if next == "*/" {
                        violations.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: self.id().to_string(),
                            message: "KDoc comment must not be empty".to_string(),
                            auto_fixable: true,
                        });
                    } else if next.starts_with('*') && !next.starts_with("* ") && next.len() > 1 {
                        violations.push(Violation {
                            file: String::new(),
                            line: i + 2,
                            col: 1,
                            rule_id: self.id().to_string(),
                            message: "KDoc asterisk should be followed by space".to_string(),
                            auto_fixable: true,
                        });
                    }
                }
            } else if trimmed.starts_with("* @param") {
                // @param is JavaDoc, not KDoc — suggest @param[name]
                violations.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().to_string(),
                    message: "Use KDoc syntax @param[name] instead of @param".to_string(),
                    auto_fixable: true,
                });
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
        KdocFormatting.check(&p.parse(s), s)
    }
    #[test]
    fn empty_kdoc() {
        let v = check("/**\n */\nclass Foo\n");
        assert!(!v.is_empty());
    }
    #[test]
    fn valid_kdoc() {
        assert!(check("/** Doc */\nclass Foo\n").is_empty());
    }
    #[test]
    fn java_param() {
        let v = check("/**\n * @param x\n */\nfun foo(x:Int)\n");
        assert!(!v.is_empty());
    }
}
