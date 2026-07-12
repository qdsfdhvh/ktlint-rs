use crate::rules::{Rule, Violation};

/// Context-sensitive indentation check (JVM-compatible).
/// Tracks a stack of expected indent levels per brace block.
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
        let is = self.indent_size;
        let lines: Vec<&str> = source.lines().collect();
        let mut stack: Vec<usize> = Vec::new();
        let mut in_block_comment = false;
        
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let spaces = line.len() - trimmed.len();

            // Track block comments across lines
            if trimmed.starts_with("/*") {
                in_block_comment = true;
            }
            if in_block_comment {
                if trimmed.ends_with("*/") {
                    in_block_comment = false;
                }
                continue;
            }

            // Skip: blank, comments, annotations, KDoc continuation
            if trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with('@')
                || trimmed == "*/"
                || trimmed.starts_with("* ")
                || trimmed.starts_with("*/")
            {
                continue;
            }

            // Expected indent = top of stack, or 0 if stack empty
            let expected = stack.last().copied().unwrap_or(0);

            // --- Closing brace ---
            if trimmed == "}" {
                // Closing brace: indent at the level BEFORE the matching {, not current
                let parent = stack.last().copied().unwrap_or(0).saturating_sub(is);
                if spaces != parent {
                    violations.push(violation(
                        self.id(), i + 1, spaces, parent,
                    ));
                }
                stack.pop();
                continue;
            }

            // --- Opening brace ---
            let opens_block = trimmed.ends_with('{')
                && !trimmed.ends_with("${")      // skip string templates
                && !trimmed.starts_with("//");   // skip comments

            // --- Indent check for non-brace, non-closing lines ---
            // Only flag if indent doesn't match expected and isn't a continuation
            if spaces != expected {
                // Allow continuation: deeper indent that's a multiple of indent_size
                let is_continuation = spaces % is == 0;

                if !is_continuation {
                    violations.push(violation(self.id(), i + 1, spaces, expected));
                }
            }

            // After processing this line, if it opens a block, push expected indent
            if opens_block {
                // Expected indent for content is current indent + indent_size
                let next_expected = spaces + is;
                stack.push(next_expected);
            }

                    }

        violations
    }
}

fn violation(rule_id: &str, line: usize, actual: usize, expected: usize) -> Violation {
    Violation {
        file: String::new(),
        line,
        col: 1,
        rule_id: rule_id.into(),
        message: format!(
            "Unexpected indentation ({}) (should be {})",
            actual, expected
        ),
        auto_fixable: true,
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
        assert!(check("fun a() {\n    val x = 1\n}\n", 4).is_empty());
    }

    #[test]
    fn wrong_indent() {
        assert!(!check("fun a() {\n  val x = 1\n}\n", 4).is_empty());
    }

    #[test]
    fn nested_blocks() {
        assert!(check("class Foo {\n    fun bar() {\n        val x = 1\n    }\n}\n", 4).is_empty());
    }

    #[test]
    fn closing_brace_at_parent_level() {
        assert!(check("fun f() {\n    if (x) {\n        y()\n    }\n}\n", 4).is_empty());
    }

    #[test]
    fn continuation_allowed() {
        // Parameter continuation
        assert!(check("fun f(\n        a: Int,\n        b: Int\n) {}\n", 4).is_empty());
    }
}
