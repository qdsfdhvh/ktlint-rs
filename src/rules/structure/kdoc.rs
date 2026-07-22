//! KDOC rules — KDoc positioning + formatting.
//!
//! Checks:
//! - KDoc inside blocks / argument lists (not allowed)
//! - Empty KDoc comments
//! - Asterisk spacing
//! - @param → @param[name] syntax
use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

pub struct KdocFormatting;

impl Rule for KdocFormatting {
    fn id(&self) -> &'static str {
        "standard:kdoc"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        let mut in_kdoc = false;
        let mut kdoc_start_line = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("/**") {
                in_kdoc = true;
                kdoc_start_line = i;

                if trimmed.ends_with("*/") && trimmed.len() > 4 {
                    // Single-line /** ... */ — check location
                    // Allow KDoc on private/internal declarations at file scope
                    let next_is_private = i + 1 < lines.len() && {
                        let n = lines[i + 1].trim();
                        n.starts_with("private ")
                            || n.starts_with("internal ")
                            || n.starts_with("protected ")
                    };
                    if !next_is_private && is_inside_block(&lines, i) {
                        violations.push(kdoc_violation(
                            self.id(),
                            i + 1,
                            "KDoc is not allowed here",
                        ));
                    }
                    in_kdoc = false;
                }
                continue;
            }

            // @param without name (JavaDoc style) — checked both inside and after KDoc
            // @param without name: skip — @param name is valid Kotlin KDoc syntax.
            // JVM ktlint does NOT flag @param without [name].

            if in_kdoc {
                if trimmed == "*/" && i == kdoc_start_line + 1 {
                    // Empty KDoc: /** followed by */
                    violations.push(kdoc_violation(
                        self.id(),
                        kdoc_start_line + 1,
                        "KDoc comment must not be empty",
                    ));
                } else if trimmed.starts_with('*')
                    && !trimmed.starts_with("* ")
                    && !trimmed.starts_with("*/")
                    && trimmed.len() > 1
                {
                    // Asterisk without space
                    violations.push(kdoc_violation(
                        self.id(),
                        i + 1,
                        "KDoc asterisk should be followed by space",
                    ));
                }

                if trimmed.contains("*/") {
                    // End of KDoc — check location
                    if is_inside_block(&lines, kdoc_start_line) {
                        violations.push(kdoc_violation(
                            self.id(),
                            kdoc_start_line + 1,
                            "KDoc is not allowed here",
                        ));
                    }
                    in_kdoc = false;
                }
                continue;
            }

            // @param name is valid KDoc syntax — JVM ktlint does NOT flag this
        }

        violations
    }
}

fn is_inside_block(lines: &[&str], kdoc_line: usize) -> bool {
    if kdoc_line == 0 {
        return false;
    }
    let kdoc_indent = lines[kdoc_line].len() - lines[kdoc_line].trim_start().len();
    let mut inside = false;
    let mut opener_is_class_like = false;
    for j in (0..kdoc_line).rev() {
        let t = lines[j].trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with("/*") || t.starts_with('*') {
            continue;
        }
        let indent = lines[j].len() - t.len();
        if indent <= kdoc_indent {
            if t == "}" {
                inside = false;
                opener_is_class_like = false;
            } else if t.contains('{') && !inside {
                inside = true;
                // Check if the opening brace is from a class-like declaration.
                // KDoc at class body level (documenting members) is OK.
                // KDoc inside function bodies, control flow, etc. is flagged.
                opener_is_class_like = is_class_like_opener(t);
                if opener_is_class_like {
                    // Don't flag yet — check if there's content between the {{
                    // and the KDoc at the KDoc's indent level. If there IS content,
                    // it means the KDoc is deeper inside a nested scope.
                    break;
                }
                // Not class-like (function body, if, when, etc.) — KDoc is inside
                // a non-class block, which is NOT allowed.
                return true;
            }
        }
    }
    // If we found a class-like opener, check if there's content at KDoc's
    // indent between the opener and the KDoc.
    if inside && opener_is_class_like {
        // Scan forward from after the opener to before KDoc.
        // If we find a non-declaration line at kdoc_indent, KDoc is nested → flag.
        let mut found_opener = false;
        for j in 0..kdoc_line {
            let t = lines[j].trim();
            let indent = lines[j].len() - t.len();
            if t.contains('{') && indent <= kdoc_indent && !found_opener {
                found_opener = true;
                continue;
            }
            if found_opener
                && indent == kdoc_indent
                && !t.is_empty()
                && !t.starts_with("//")
                && !t.starts_with("/*")
                && !t.starts_with('*')
                || t == "}" && !is_declaration_or_modifier(t)
            {
                return true;
            }
        }
        return false;
    }
    inside
}

/// Check if a line opens a class-like body (class, object, interface, enum, companion).
fn is_class_like_opener(line: &str) -> bool {
    line.starts_with("class ")
        || line.starts_with("interface ")
        || line.starts_with("object ")
        || line.starts_with("enum class ")
        || line.starts_with("companion object ")
        || line.starts_with("data class ")
        || line.starts_with("sealed class ")
        || line.starts_with("sealed interface ")
        || line.starts_with("abstract class ")
        || line.starts_with("open class ")
        || line.starts_with("inline class ")
        || line.starts_with("value class ")
        || line.starts_with("expect class ")
        || line.starts_with("actual class ")
        || line.starts_with("annotation class ")
}

/// Check if a trimmed line looks like a declaration or modifier (annotation, visibility).
fn is_declaration_or_modifier(line: &str) -> bool {
    if line == "}" {
        return true;
    }
    line.starts_with("fun ")
        || line.starts_with("val ")
        || line.starts_with("var ")
        || line.starts_with("class ")
        || line.starts_with("object ")
        || line.starts_with("interface ")
        || line.starts_with("enum class ")
        || line.starts_with("companion object ")
        || line.starts_with("data class ")
        || line.starts_with("sealed class ")
        || line.starts_with("typealias ")
        || line.starts_with("init ")
        || line.starts_with("constructor(")
        || line.starts_with('@')
        || line.starts_with("private ")
        || line.starts_with("internal ")
        || line.starts_with("protected ")
        || line.starts_with("public ")
        || line.starts_with("suspend ")
        || line.starts_with("operator ")
        || line.starts_with("override ")
        || line.starts_with("abstract ")
        || line.starts_with("open ")
        || line.starts_with("tailrec ")
        || line.starts_with("external ")
        || line.starts_with("inline ")
        || line.starts_with("expect ")
        || line.starts_with("actual ")
}

fn kdoc_violation(rule_id: &str, line: usize, msg: &str) -> Violation {
    Violation {
        file: String::new(),
        line,
        col: 1,
        rule_id: rule_id.into(),
        message: msg.into(),
        auto_fixable: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        KdocFormatting.check(&p.parse(s), s)
    }

    #[test]
    fn kdoc_before_declaration_ok() {
        assert!(check("/** doc */\nfun f() {}\n").is_empty());
    }

    #[test]
    fn kdoc_inside_block_bad() {
        assert!(!check("fun f() {\n    /** doc */\n    val x = 1\n}\n").is_empty());
    }

    #[test]
    fn empty_kdoc() {
        assert!(!check("/**\n */\nclass Foo\n").is_empty());
    }

    #[test]
    fn valid_kdoc() {
        assert!(check("/** Doc */\nclass Foo\n").is_empty());
    }

    #[test]
    fn java_param_not_flagged() {
        // @param name is valid Kotlin KDoc syntax — JVM ktlint does NOT flag this
        assert!(check("/**\n * @param x\n */\nfun foo(x:Int)\n").is_empty());
    }
}
