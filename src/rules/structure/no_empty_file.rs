//! standard:no-empty-file — flag empty Kotlin files.

use crate::rules::{Rule, Violation};

pub struct NoEmptyFile;

impl Rule for NoEmptyFile {
    fn id(&self) -> &'static str {
        "standard:no-empty-file"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let trimmed = source.trim();
        if trimmed.is_empty() || trimmed == "//" || trimmed == "/* */" || trimmed.len() <= 2 {
            vec![Violation {
                file: String::new(),
                line: 1,
                col: 1,
                rule_id: self.id().to_string(),
                message: "File is empty".to_string(),
                auto_fixable: false, // can't auto-fix — file should be deleted
            }]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        NoEmptyFile.check(&tree, source)
    }

    #[test]
    fn empty_file() {
        let v = check("");
        assert!(!v.is_empty());
    }

    #[test]
    fn non_empty_file() {
        assert!(check("val x = 1\n").is_empty());
    }

    #[test]
    fn whitespace_only() {
        let v = check("  \n  \n");
        assert!(!v.is_empty());
    }
}
