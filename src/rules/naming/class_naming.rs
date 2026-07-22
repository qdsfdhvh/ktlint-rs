//! standard:class-naming — class/enum/object names must be PascalCase.

use crate::rules::{Rule, Violation};

pub struct ClassNaming;

impl Rule for ClassNaming {
    fn id(&self) -> &'static str {
        "standard:class-naming"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // Check class declarations
            if let Some(name) = extract_name_after_keyword(trimmed, "class ") {
                if !is_pascal_case(&name) && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    violations.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: trimmed.find("class ").unwrap_or(0) + 7,
                        rule_id: self.id().to_string(),
                        message: format!("Class name \"{}\" should be PascalCase", name),
                        auto_fixable: false,
                    });
                }
            }

            // Enum declarations
            if let Some(name) = extract_name_after_keyword(trimmed, "enum ") {
                // Skip 'enum class Foo' — the 'class' keyword is not an enum name
                if name == "class" || name == "interface" {
                    continue;
                }
                if !is_pascal_case(&name) {
                    violations.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: trimmed.find("enum ").unwrap_or(0) + 6,
                        rule_id: self.id().to_string(),
                        message: format!("Enum name \"{}\" should be PascalCase", name),
                        auto_fixable: false,
                    });
                }
            }

            // Object declarations
            if let Some(name) = extract_name_after_keyword(trimmed, "object ") {
                if !is_pascal_case(&name) {
                    violations.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: trimmed.find("object ").unwrap_or(0) + 8,
                        rule_id: self.id().to_string(),
                        message: format!("Object name \"{}\" should be PascalCase", name),
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
    // Take the first token (name)
    let name = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn is_pascal_case(s: &str) -> bool {
    s.chars().next().map_or(false, |c| c.is_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        ClassNaming.check(&tree, source)
    }

    #[test]
    fn pascal_case_class() {
        assert!(check("class MyViewModel\n").is_empty());
    }

    #[test]
    fn snake_case_class() {
        let v = check("class my_view_model\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:class-naming");
    }

    #[test]
    fn pascal_case_enum() {
        assert!(check("enum Color { RED, GREEN }\n").is_empty());
    }
}
