//! standard:function-naming — function names must be camelCase.

use crate::rules::{Rule, Violation};

pub struct FunctionNaming;

impl Rule for FunctionNaming {
    fn id(&self) -> &'static str {
        "standard:function-naming"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            if let Some(name) = extract_name_after_keyword(trimmed, "fun ") {
                if is_operator_function(&name) || name.starts_with("test_") {
                    continue;
                }
                if !is_camel_case(&name) {
                    violations.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: trimmed.find("fun ").unwrap_or(0) + 5,
                        rule_id: self.id().to_string(),
                        message: format!("Function name \"{}\" should be camelCase", name),
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

fn is_operator_function(name: &str) -> bool {
    matches!(
        name,
        "plus"
            | "minus"
            | "times"
            | "div"
            | "rem"
            | "compareTo"
            | "get"
            | "set"
            | "contains"
            | "invoke"
            | "rangeTo"
            | "iterator"
    )
}

fn is_camel_case(s: &str) -> bool {
    s.chars().next().map_or(false, |c| c.is_lowercase()) && !s.contains('_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        FunctionNaming.check(&tree, source)
    }

    #[test]
    fn camel_case_function() {
        assert!(check("fun myFunction()\n").is_empty());
    }

    #[test]
    fn pascal_case_function() {
        let v = check("fun MyFunction()\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:function-naming");
    }

    #[test]
    fn operator_function_ok() {
        assert!(check("fun plus(other: Int)\n").is_empty());
    }

    #[test]
    fn snake_case_underscore() {
        let v = check("fun my_function()\n");
        assert!(!v.is_empty());
    }
}
