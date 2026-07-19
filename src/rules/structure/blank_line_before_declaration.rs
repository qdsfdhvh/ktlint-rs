use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

/// Checks that declarations (class, function, property) are preceded by a blank
/// line unless they're the first declaration in a file/class body.
/// JVM-compatible: checks both top-level and inside class bodies.
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

        // Scan backwards from each declaration to find the previous non-blank
        // non-comment line. If it's also a declaration at the same indent, flag.
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            if !is_declaration(trimmed) {
                continue;
            }

            let current_indent = line.len().saturating_sub(trimmed.len());

            // Skip very first declaration in file
            if current_indent == 0 && i == 0 {
                continue;
            }

            // Find previous non-blank, non-comment line
            let mut prev_idx = i;
            let mut found_blank = false;
            loop {
                if prev_idx == 0 {
                    break;
                }
                prev_idx -= 1;
                let p = lines[prev_idx].trim();
                if p.is_empty() {
                    found_blank = true;
                    continue;
                }
                if p.starts_with("//")
                    || p.starts_with("/*")
                    || p.starts_with('@')
                    || p == "*/"
                    || p.starts_with("* ")
                {
                    continue;
                }
                // Found a non-blank, non-comment line
                let prev_indent = lines[prev_idx].len().saturating_sub(p.len());

                // Only flag if same indent level AND not separated by blank line
                // AND the previous line is also a declaration
                if prev_indent == current_indent && !found_blank {
                    // Special case: previous line has `{` at the end (opening brace)
                    // e.g., "class Foo {" — first declaration in a block is fine
                    if p.ends_with('{') {
                        break;
                    }
                    // Previous line must be a declaration too
                    if is_declaration(p) {
                        violations.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: "standard:blank-line-before-declaration".into(),
                            message: format!(
                                "Expected a blank line before declaration: \"{}\"",
                                if trimmed.len() > 60 {
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
                break;
            }
        }

        violations
    }
}

fn is_declaration(trimmed: &str) -> bool {
    trimmed.starts_with("fun ")
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
        || trimmed.starts_with("public fun ")
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

    #[test]
    fn inside_class_body_no_blank() {
        let v = check("class Foo {\n    fun a() {}\n    fun b() {}\n}\n");
        assert!(!v.is_empty());
    }

    #[test]
    fn inside_class_body_with_blank() {
        let v = check("class Foo {\n    fun a() {}\n\n    fun b() {}\n}\n");
        assert!(v.is_empty());
    }

    #[test]
    fn first_in_class_body_no_blank_needed() {
        let v = check("class Foo {\n    fun a() {}\n}\n");
        assert!(v.is_empty());
    }
}
