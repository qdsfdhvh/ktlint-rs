//! standard:enum-entry — each enum entry should be on its own line.

use crate::rules::{Rule, Violation};

pub struct EnumEntry;

impl Rule for EnumEntry {
    fn id(&self) -> &'static str {
        "standard:enum-entry"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("enum ") {
                // Check if enum entries span multiple lines on the opening line
                // e.g., `enum Color { RED, GREEN, BLUE }` — all on one line
                if let Some(brace_pos) = trimmed.find('{') {
                    let after_brace = &trimmed[brace_pos + 1..];
                    let entries: Vec<&str> = after_brace
                        .trim_end_matches('}')
                        .split(',')
                        .filter(|s| !s.trim().is_empty())
                        .collect();

                    if entries.len() > 1 {
                        // Multiple entries on the same line — violation
                        violations.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: brace_pos as usize + 2,
                            rule_id: self.id().to_string(),
                            message: "Each enum entry should be on its own line".to_string(),
                            auto_fixable: true,
                        });
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

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        EnumEntry.check(&tree, source)
    }

    #[test]
    fn single_line_enum_entries() {
        let v = check("enum Color { RED, GREEN, BLUE }\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:enum-entry");
    }

    #[test]
    fn multiline_enum_ok() {
        assert!(check("enum Color {\n    RED,\n    GREEN,\n    BLUE\n}\n").is_empty());
    }

    #[test]
    fn empty_enum_ok() {
        assert!(check("enum Color {}\n").is_empty());
    }
}
