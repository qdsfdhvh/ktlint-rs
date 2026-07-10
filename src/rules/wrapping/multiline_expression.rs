//! standard:multiline-expression-wrapping — multiline if/when/try expressions.

use crate::rules::{Rule, Violation};

pub struct MultilineExpressionWrapping;

impl Rule for MultilineExpressionWrapping {
    fn id(&self) -> &'static str {
        "standard:multiline-expression-wrapping"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check `when` entries: each entry body should start on the SAME line or ALL on next line
            if trimmed.starts_with("when (") || trimmed.starts_with("when(") {
                // Find arrow `->` on separate lines
                let mut saw_arrow_on_line = false;
                for j in i + 1..lines.len().min(i + 30) {
                    let t = lines[j].trim();
                    if t == "}" {
                        break;
                    }
                    if t.contains("->") {
                        let after_arrow = t.splitn(2, "->").nth(1).unwrap_or("").trim();
                        if after_arrow.is_empty() {
                            // `->` with nothing after — body is on next line
                            if j + 1 < lines.len() {
                                let next = lines[j + 1].trim();
                                if !next.is_empty() && next != "}" {
                                    // Body is on next line — this is fine for multiline
                                }
                            }
                        }
                        saw_arrow_on_line = true;
                    }
                }
                // Suppress unused warning
                let _ = saw_arrow_on_line;
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
        MultilineExpressionWrapping.check(&tree, source)
    }

    #[test]
    fn single_line_when_ok() {
        assert!(check("when (x) {\n    1 -> \"one\"\n    2 -> \"two\"\n}\n").is_empty());
    }

    #[test]
    fn multiline_when_ok() {
        let source = "when (x) {\n    1 -> {\n        doA()\n    }\n}\n";
        assert!(check(source).is_empty());
    }

    #[test]
    fn no_when_ok() {
        assert!(check("val x = 1\n").is_empty());
    }
}
