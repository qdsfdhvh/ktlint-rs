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
                    if is_inside_block(&lines, i) {
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
    for j in (0..kdoc_line).rev() {
        let t = lines[j].trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with("/*") || t.starts_with('*') {
            continue;
        }
        let indent = lines[j].len() - t.len();
        if indent <= kdoc_indent {
            if t == "}" {
                inside = false;
            } else if t.contains('{') && !inside {
                // First { we encounter at this level (after any closing }) means
                // the KDoc is at the same level as a block-opening brace
                inside = true;
                break;
            }
        }
    }
    inside
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
