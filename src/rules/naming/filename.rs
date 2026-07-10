//! standard:filename — Kotlin file names should match the top-level class.

use crate::rules::{Rule, Violation};
use std::path::Path;

pub struct Filename;

impl Rule for Filename {
    fn id(&self) -> &'static str {
        "standard:filename"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        // Find the first top-level class declaration
        for line in source.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("class ") {
                if let Some(name) = extract_first_token(rest) {
                    // The filename will be set in the engine when file path is known
                    // For now, just note if a class declaration exists
                    if name.is_empty() {
                        return vec![Violation {
                            file: String::new(),
                            line: 1,
                            col: 1,
                            rule_id: self.id().to_string(),
                            message: "Could not determine class name for filename check"
                                .to_string(),
                            auto_fixable: false,
                        }];
                    }
                    // This rule is a no-op in isolation — the engine sets the filename
                    return vec![];
                }
            }
        }
        // No class declaration — no filename convention to check
        vec![]
    }
}

impl Filename {
    /// Set file context after checking.
    pub fn check_with_file(&self, file_path: &Path, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Find the first top-level class
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("class ") {
                if let Some(class_name) = extract_first_token(rest) {
                    let expected_file = format!("{}.kt", class_name);
                    let actual_file = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                    if actual_file != expected_file {
                        violations.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: trimmed.find("class ").unwrap_or(0) + 7,
                            rule_id: self.id().to_string(),
                            message: format!(
                                "File name \"{}\" does not match class \"{}\" (expected \"{}\")",
                                actual_file, class_name, expected_file
                            ),
                            auto_fixable: false,
                        });
                    }
                    return violations;
                }
            }
        }

        violations
    }
}

fn extract_first_token(s: &str) -> Option<String> {
    let name: String = s
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    #[test]
    fn check_returns_empty() {
        let mut parser = KotlinParser::new();
        let tree = parser.parse("class Foo\n");
        let v = Filename.check(&tree, "class Foo\n");
        assert!(v.is_empty());
    }

    #[test]
    fn check_with_file_matches() {
        let v = Filename.check_with_file(Path::new("Foo.kt"), "class Foo\n");
        assert!(v.is_empty());
    }

    #[test]
    fn check_with_file_mismatch() {
        let v = Filename.check_with_file(Path::new("Bar.kt"), "class Foo\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:filename");
    }
}
