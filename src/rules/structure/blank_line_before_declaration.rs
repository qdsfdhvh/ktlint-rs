use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

/// Checks that declarations (class, function, property) are preceded by a blank
/// line unless they're the first declaration in a file/class body.
pub struct BlankLineBeforeDeclaration;

impl Rule for BlankLineBeforeDeclaration {
    fn id(&self) -> &'static str {
        "standard:blank-line-before-declaration"
    }

    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        if lines.len() < 2 {
            return violations;
        }

        let mut prev_line_is_empty = true; // Treat file start as "blank"
        let mut prev_line_is_decl = false;
        let mut prev_indent = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let current_indent = line.len().saturating_sub(trimmed.len());

            // Detect declarations: fun / val / var / class / object / interface /
            // enum class / typealias / constructor / init / companion object at
            // indent level <= previous (top-level or after closing brace).
            let is_decl = trimmed.starts_with("fun ")
                || trimmed.starts_with("val ")
                || trimmed.starts_with("var ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("object ")
                || trimmed.starts_with("interface ")
                || trimmed.starts_with("enum class ")
                || trimmed.starts_with("typealias ")
                || trimmed.starts_with("constructor(")
                || trimmed.starts_with("init ")
                || trimmed.starts_with("companion object ")
                || trimmed.starts_with("data class ")
                || trimmed.starts_with("sealed class ")
                || trimmed.starts_with("sealed interface ")
                || trimmed.starts_with("abstract class ")
                || trimmed.starts_with("open class ")
                || trimmed.starts_with("annotation class ")
                || trimmed.starts_with("expect class ")
                || trimmed.starts_with("actual class ")
                || trimmed.starts_with("inline class ")
                || trimmed.starts_with("value class ")
                || trimmed.starts_with("suspend fun ")
                || trimmed.starts_with("operator fun ")
                || trimmed.starts_with("infix fun ")
                || trimmed.starts_with("tailrec fun ")
                || trimmed.starts_with("external fun ")
                || trimmed.starts_with("inline fun ")
                || trimmed.starts_with("override fun ")
                || trimmed.starts_with("private fun ")
                || trimmed.starts_with("internal fun ")
                || trimmed.starts_with("protected fun ")
                || trimmed.starts_with("public fun ");

            // Only top-level declarations (indent == 0) require a blank line.
            if is_decl
                && current_indent == 0
                && prev_indent == 0
                && !prev_line_is_empty
                && prev_line_is_decl
            {
                // Check if this is the first thing after an opening brace or
                // a comment; if so, don't flag.
                if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("/*") {
                    violations.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message: format!(
                            "Expected a blank line before declaration: \"{}\"",
                            if trimmed.len() > 60 {
                                // Safe UTF-8 truncation at char boundary
                                let end = trimmed
                                    .char_indices()
                                    .nth(57)
                                    .map(|(i, _)| i)
                                    .unwrap_or(trimmed.len());
                                &trimmed[..end]
                            } else {
                                trimmed
                            }
                        ),
                        auto_fixable: true,
                    });
                }
            }

            if trimmed.is_empty() {
                prev_line_is_empty = true;
            } else {
                prev_line_is_empty = false;
            }
            prev_line_is_decl = is_decl;
            prev_indent = current_indent;
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        BlankLineBeforeDeclaration.check(&p.parse(src), src)
    }

    #[test]
    fn no_blank_between_declarations() {
        let v = check("fun a() {}\nfun b() {}");
        assert!(!v.is_empty());
    }

    #[test]
    fn blank_between_declarations() {
        let v = check("fun a() {}\n\nfun b() {}");
        assert!(v.is_empty());
    }

    #[test]
    fn first_declaration_no_blank() {
        let v = check("fun a() {}");
        assert!(v.is_empty());
    }
}
