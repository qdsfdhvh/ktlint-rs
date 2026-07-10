//! standard:string-template-indent — multiline strings should use trimIndent/trimMargin.

use crate::rules::{Rule, Violation};

pub struct StringTemplateIndent;

impl Rule for StringTemplateIndent {
    fn id(&self) -> &'static str {
        "standard:string-template-indent"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        let mut in_multiline_string = false;
        let mut string_start_line = 0;
        let mut found_trim_call = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            if !in_multiline_string {
                // Detect start of a multiline string
                if trimmed.contains("\"\"\"") {
                    // Check if it's more than just the opening
                    let count = trimmed.matches("\"\"\"").count();
                    if count >= 2 {
                        // Opens and closes on same line — not multiline
                        continue;
                    }
                    in_multiline_string = true;
                    string_start_line = i;
                    found_trim_call =
                        trimmed.contains(".trimIndent()") || trimmed.contains(".trimMargin()");
                }
            } else {
                // Inside multiline string
                if trimmed.contains("\"\"\"") {
                    // Closing the multiline string
                    in_multiline_string = false;
                    if !found_trim_call {
                        // Check if the closing line or next line has trim call
                        let rest = &source[line.find("\"\"\"").unwrap_or(0)..];
                        if rest.contains(".trimIndent()") || rest.contains(".trimMargin()") {
                            found_trim_call = true;
                        } else {
                            violations.push(Violation {
                                file: String::new(),
                                line: string_start_line + 1,
                                col: 1,
                                rule_id: self.id().to_string(),
                                message: "Multiline string literal should use trimIndent() or trimMargin()".to_string(),
                                auto_fixable: false,
                            });
                        }
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
        StringTemplateIndent.check(&tree, source)
    }

    #[test]
    fn multiline_with_trim_indent() {
        let source = "val s = \"\"\"\n    hello\n    \"\"\".trimIndent()\n";
        assert!(check(source).is_empty());
    }

    #[test]
    fn multiline_without_trim() {
        let source = "val s = \"\"\"\n    hello\n    \"\"\"\n";
        let v = check(source);
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:string-template-indent");
    }

    #[test]
    fn single_line_string_ok() {
        assert!(check("val s = \"hello\"\n").is_empty());
    }
}
