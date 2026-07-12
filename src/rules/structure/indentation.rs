use crate::rules::{Rule, Violation};

/// Checks that lines are indented by multiples of indent_size from .editorconfig.
/// Does NOT handle continuation indents (wrapping, multi-line expressions)
/// — those are handled by dedicated wrapping rules.
pub struct Indentation {
    indent_size: usize,
}

impl Indentation {
    pub fn new(indent_size: usize) -> Self {
        Self { indent_size }
    }
}

impl Rule for Indentation {
    fn id(&self) -> &'static str {
        "standard:indent"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let indent_size = self.indent_size;
        let lines: Vec<&str> = source.lines().collect();
        let mut expected_indent = 0usize;
        let mut in_block_comment = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Track block comment boundaries — skip contents entirely
            if trimmed.starts_with("/*") {
                in_block_comment = true;
            }
            if in_block_comment {
                if trimmed.ends_with("*/") {
                    in_block_comment = false;
                }
                continue;
            }

            // Skip line comments and annotation lines (may appear at any indent)
            if trimmed.starts_with("//") || trimmed.starts_with('@') {
                continue;
            }

            let spaces = line.len() - trimmed.len();

            // Reject tabs
            if line.starts_with('\t') {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Unexpected tab character(s)".into(),
                    auto_fixable: true,
                });
                continue;
            }

            // Check indent: must be a multiple of indent_size
            if spaces > 0 && spaces % indent_size != 0 {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: format!(
                        "Unexpected indentation ({}) — should be a multiple of {}",
                        spaces, indent_size
                    ),
                    auto_fixable: true,
                });
            }

            // Adjust block level for NEXT line based on braces
            if trimmed == "}" {
                expected_indent = expected_indent.saturating_sub(indent_size);
            }
            if trimmed.ends_with('{') {
                expected_indent += indent_size;
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str, indent_size: usize) -> Vec<Violation> {
        Indentation::new(indent_size).check(
            &crate::parser::KotlinParser::new().parse(src),
            src,
        )
    }

    #[test]
    fn correct_indent() {
        let v = check("fun a() {\n    val x = 1\n}\n", 4);
        assert!(v.is_empty(), "expected no violations: {:#?}", v);
    }

    #[test]
    fn wrong_indent() {
        let v = check("fun a() {\n  val x = 1\n}\n", 4);
        assert!(!v.is_empty(), "expected violations for 2-space indent with indent_size=4");
    }

    #[test]
    fn block_comment_skipped() {
        // Lines inside a block comment are not checked
        let v = check("/*\n * comment body\n */\nfun a() {\n    val x = 1\n}\n", 4);
        assert!(v.is_empty(), "block comment lines should be skipped: {:#?}", v);
    }

    #[test]
    fn zero_indent_top_level() {
        let v = check("fun a() {}\nfun b() {}\n", 4);
        assert!(v.is_empty(), "top-level declarations with 0 indent are fine: {:#?}", v);
    }

    #[test]
    fn nested_blocks() {
        let v = check(
            "class Foo {\n    fun bar() {\n        val x = 1\n    }\n}\n",
            4,
        );
        assert!(v.is_empty(), "nested blocks with correct indent: {:#?}", v);
    }

    #[test]
    fn mix_of_tabs() {
        let v = check("\tfun a() {}\n", 4);
        assert!(!v.is_empty(), "tabs should be rejected");
    }

    #[test]
    fn line_comment_skipped() {
        let v = check("fun a() {\n    // this is a comment\n    val x = 1\n}\n", 4);
        assert!(v.is_empty(), "line comments should be skipped: {:#?}", v);
    }
}
