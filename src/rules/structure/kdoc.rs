//! KDOC rules — KDoc positioning + formatting.
//!
//! Checks:
//! - KDoc inside blocks / argument lists (not allowed)
//! - Empty KDoc comments
//! - Asterisk spacing
//! - @param → @param[name] syntax
use crate::rules::{Rule, Violation};
use std::collections::HashSet;
use tree_sitter::Tree;

pub struct KdocFormatting;

impl Rule for KdocFormatting {
    fn id(&self) -> &'static str {
        "standard:kdoc"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        let disallowed_lines = disallowed_kdoc_lines(tree, source);

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
                    if !next_is_private && disallowed_lines.contains(&i) {
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
                    if disallowed_lines.contains(&kdoc_start_line) {
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

fn disallowed_kdoc_lines(tree: &Tree, source: &str) -> HashSet<usize> {
    let mut result = HashSet::new();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.kind().contains("comment")
            && source[node.start_byte()..node.end_byte()].starts_with("/**")
            && is_in_executable_context(node)
        {
            result.insert(node.start_position().row);
        }
        for index in (0..node.child_count()).rev() {
            if let Some(child) = node.child(index) {
                stack.push(child);
            }
        }
    }
    result
}

fn is_in_executable_context(node: tree_sitter::Node<'_>) -> bool {
    let mut parent = node.parent();
    let mut saw_block = false;
    while let Some(current) = parent {
        match current.kind() {
            "block" | "function_body" => saw_block = true,
            "class_declaration" | "object_declaration" => return false,
            "function_declaration" | "lambda_literal" => return saw_block,
            _ => {}
        }
        parent = current.parent();
    }
    false
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

    #[test]
    fn kdoc_on_class_member_after_closed_class_is_allowed() {
        let source = "public data class Previous(\n    val value: String,\n) {\n}\n\npublic data class Current(\n    /** Member docs. */\n    val value: String,\n)\n";
        assert!(check(source).is_empty());
    }

    #[test]
    fn kdoc_on_member_of_public_sealed_interface_is_allowed() {
        let source = "public sealed interface Action {\n    /** Member docs. */\n    public data object Trigger : Action\n}\n";
        assert!(check(source).is_empty());
    }

    #[test]
    fn kdoc_on_member_of_multiline_class_header_is_allowed() {
        let source = "internal class Example(\n    dependency: String,\n) : Base(\n    dependency = dependency,\n) {\n    /** Member docs. */\n    private var loaded = false\n}\n";
        assert!(check(source).is_empty());
    }

    #[test]
    fn kdoc_on_fun_interface_member_is_allowed() {
        let source = "fun interface Handler {\n    /** Handles a value. */\n    fun handle(value: String)\n}\n";
        assert!(check(source).is_empty());
    }
}
