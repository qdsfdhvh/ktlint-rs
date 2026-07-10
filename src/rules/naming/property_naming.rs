//! standard:property-naming — val/var/camelCase, const/UPPER_SNAKE.

use crate::rules::{Rule, Violation};

pub struct PropertyNaming;

impl Rule for PropertyNaming {
    fn id(&self) -> &'static str {
        "standard:property-naming"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // const val/var: UPPER_SNAKE_CASE
            if let Some(name) = extract_name_after_keyword(trimmed, "const val ") {
                if !is_upper_snake(&name) {
                    violations.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: trimmed.find("const val ").unwrap_or(0) + 11,
                        rule_id: self.id().to_string(),
                        message: format!("Const property \"{}\" should be UPPER_SNAKE_CASE", name),
                        auto_fixable: false,
                    });
                }
                continue;
            }
            if let Some(name) = extract_name_after_keyword(trimmed, "const var ") {
                if !is_upper_snake(&name) {
                    violations.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: trimmed.find("const var ").unwrap_or(0) + 11,
                        rule_id: self.id().to_string(),
                        message: "Const var should be UPPER_SNAKE_CASE".to_string(),
                        auto_fixable: false,
                    });
                }
                continue;
            }

            // val/var: camelCase
            if let Some(name) = extract_name_after_keyword(trimmed, "val ") {
                let name = name.split(':').next().unwrap_or(&name);
                if !is_camel_case(name) {
                    violations.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: trimmed.find("val ").unwrap_or(0) + 5,
                        rule_id: self.id().to_string(),
                        message: format!("Property name \"{}\" should be camelCase", name),
                        auto_fixable: false,
                    });
                }
                continue;
            }
            if let Some(name) = extract_name_after_keyword(trimmed, "var ") {
                let name = name.split(':').next().unwrap_or(&name);
                if !is_camel_case(name) {
                    violations.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: trimmed.find("var ").unwrap_or(0) + 5,
                        rule_id: self.id().to_string(),
                        message: format!("Variable name \"{}\" should be camelCase", name),
                        auto_fixable: false,
                    });
                }
            }
        }

        violations
    }
}

fn extract_name_after_keyword(line: &str, keyword: &str) -> Option<String> {
    let rest = line.strip_prefix(keyword)?;
    let rest = rest.trim_start();
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn is_camel_case(s: &str) -> bool {
    s.chars().next().map_or(false, |c| c.is_lowercase()) && !s.contains('_')
}

fn is_upper_snake(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_uppercase() || c.is_numeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        PropertyNaming.check(&tree, source)
    }

    #[test]
    fn camel_case_val() {
        assert!(check("val myProperty = 1\n").is_empty());
    }

    #[test]
    fn upper_case_const() {
        assert!(check("const val MAX_COUNT = 100\n").is_empty());
    }

    #[test]
    fn wrong_const_case() {
        let v = check("const val maxCount = 100\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:property-naming");
    }

    #[test]
    fn pascal_case_var() {
        let v = check("var MyVar = 1\n");
        assert!(!v.is_empty());
    }
}
