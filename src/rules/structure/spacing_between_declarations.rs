//! standard:spacing-between-declarations — enforce blank lines between declarations.

use crate::rules::{Rule, Violation};

pub struct SpacingBetweenDeclarations;

impl Rule for SpacingBetweenDeclarations {
    fn id(&self) -> &'static str {
        "standard:spacing-between-declarations"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        let declaration_keywords = [
            "fun ", "val ", "var ", "class ", "object ", "interface ", "enum ",
            "typealias ", "data class ", "sealed class ", "sealed interface ",
            "abstract class ", "open class ", "inline class ", "value class ",
            "annotation class ", "companion object", "init {", "constructor(",
        ];

        for i in 0..lines.len() {
            let trimmed = lines[i].trim();

            // Check if this line starts a declaration
            let is_decl = declaration_keywords
                .iter()
                .any(|kw| trimmed.starts_with(kw) || trimmed.starts_with(&format!("@Suppress")))
                || (trimmed.contains("fun ") && !trimmed.contains("\"") && !trimmed.contains("//"));

            // More precise: check for top-level declarations (not inside blocks)
            if is_decl && i > 0 {
                let prev_trimmed = lines[i - 1].trim();
                // Empty line between declarations is required
                if !prev_trimmed.is_empty() {
                    // Previous line is not blank — might still be OK if it's a comment
                    if !prev_trimmed.starts_with("//") && !prev_trimmed.starts_with("/*") {
                        // Check if previous line ends a declaration or block
                        if prev_trimmed == "}" || prev_trimmed.ends_with(')') {
                            // Add a blank line before this declaration
                            // (for top-level declarations only, not inside classes)
                            if trimmed.starts_with("fun ") || trimmed.starts_with("class ") {
                                violations.push(Violation {
                                    file: String::new(),
                                    line: i + 1,
                                    col: 1,
                                    rule_id: self.id().to_string(),
                                    message: "Missing blank line before declaration"
                                        .to_string(),
                                    auto_fixable: true,
                                });
                            }
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
        SpacingBetweenDeclarations.check(&tree, source)
    }

    #[test]
    fn single_declaration_ok() {
        assert!(check("val x = 1\n").is_empty());
    }

    #[test]
    fn declarations_with_blank_line() {
        assert!(check("val x = 1\n\nfun foo()\n").is_empty());
    }
}
