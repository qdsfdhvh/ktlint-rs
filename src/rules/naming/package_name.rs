//! standard:package-name — package name must be lowercase dot-separated.

use crate::rules::{Rule, Violation};

pub struct PackageName;

impl Rule for PackageName {
    fn id(&self) -> &'static str {
        "standard:package-name"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("package ") {
                let pkg = rest.trim();
                // Skip annotations or comments that might follow
                // Package name should be lowercase with dots, numbers allowed
                let valid = pkg
                    .chars()
                    .take_while(|c| !c.is_whitespace() && *c != '/' && *c != ';')
                    .all(|c| c.is_lowercase() || c.is_numeric() || c == '.' || c == '_');

                if !valid && !pkg.is_empty() {
                    return vec![Violation {
                        file: String::new(),
                        line: i + 1,
                        col: trimmed.find("package ").unwrap_or(0) + 9,
                        rule_id: self.id().to_string(),
                        message: format!(
                            "Package name \"{}\" should be lowercase (only dots, digits, underscores allowed)",
                            pkg.split_whitespace().next().unwrap_or(pkg)
                        ),
                        auto_fixable: false,
                    }];
                }
                return vec![];
            }
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        PackageName.check(&tree, source)
    }

    #[test]
    fn lowercase_package_ok() {
        assert!(check("package com.example.foo\n").is_empty());
    }

    #[test]
    fn uppercase_package_bad() {
        let v = check("package com.Example.Foo\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:package-name");
    }

    #[test]
    fn no_package_declaration() {
        assert!(check("class Foo\n").is_empty());
    }

    #[test]
    fn single_segment_package() {
        assert!(check("package foo\n").is_empty());
    }
}
