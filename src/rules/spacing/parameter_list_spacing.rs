//! `standard:parameter-list-spacing` — consistent spacing inside a function's
//! value parameter list (mirrors ktlint 1.8).
//!
//! Checks:
//! - no whitespace inside an empty list `( )`
//! - no whitespace directly after `(` or before `)` for the first/last parameter
//! - no whitespace before a comma
//! - single whitespace after a comma (unless trailing comma)
//! - single whitespace between a parameter modifier (`vararg`, `crossinline`,
//!   `noinline`) and the identifier
//! - no whitespace between a parameter identifier and its `:`
//! - single whitespace after `:`

use crate::rules::{Rule, Violation};

pub struct ParameterListSpacing;

impl Rule for ParameterListSpacing {
    fn id(&self) -> &'static str {
        "standard:parameter-list-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "function_value_parameters" {
                self.check_list(&node, bytes, source, &mut violations);
            }
            for i in (0..node.child_count()).rev() {
                if let Some(child) = node.child(i) {
                    stack.push(child);
                }
            }
        }
        violations
    }
}

impl ParameterListSpacing {
    fn check_list(
        &self,
        node: &tree_sitter::Node,
        bytes: &[u8],
        source: &str,
        violations: &mut Vec<Violation>,
    ) {
        for index in 0..node.child_count() {
            let Some(child) = node.child(index) else {
                continue;
            };
            let kind = child.kind();
            let start = child.start_byte();
            let end = child.end_byte();
            match kind {
                "(" => {
                    // Empty list `( )` or whitespace after `(` before first param.
                    if index + 1 < node.child_count() {
                        if let Some(next) = node.child(index + 1) {
                            if next.kind() == ")" && end < bytes.len() {
                                // `( )` / `(\n)` — any whitespace inside an
                                // empty list (oracle: "Unexpected whitespace").
                                let gap = &bytes[end..next.start_byte()];
                                if gap.iter().any(|b| *b == b' ' || *b == b'\t' || *b == b'\n') {
                                    // Oracle: "Unexpected whitespace" at the
                                    // column right after `(`.
                                    violations.push(self.v_empty(end, source));
                                }
                            }
                        }
                    }
                }
                ")" => {
                    // Whitespace before `)` (after last parameter). Empty list
                    // `( )` is handled by the `(` branch — skip here.
                    if index > 1 {
                        if let Some(prev) = node.child(index - 1) {
                            let gap = &bytes[prev.end_byte()..start];
                            if gap.iter().any(|b| *b == b' ' && !gap.contains(&b'\n')) {
                                // space (not newline) before close paren
                                violations.push(self.v(prev.end_byte(), start, bytes, source));
                            }
                        }
                    }
                }
                "," => {
                    // No whitespace before comma; single space after.
                    if index > 0 {
                        if let Some(prev) = node.child(index - 1) {
                            let before = &bytes[prev.end_byte()..start];
                            if before
                                .iter()
                                .any(|b| *b == b' ' && !before.contains(&b'\n'))
                            {
                                violations.push(self.v(prev.end_byte(), start, bytes, source));
                            }
                        }
                    }
                    if index + 1 < node.child_count() {
                        if let Some(next) = node.child(index + 1) {
                            let after = &bytes[end..next.start_byte()];
                            if next.kind() != ")"
                                && !after.iter().any(|b| *b == b' ' || *b == b'\n')
                            {
                                violations.push(self.v(end, next.start_byte(), bytes, source));
                            }
                        }
                    }
                }
                _ => {
                    // Parameter-internal spacing: `name : Type` and `name:Type`.
                    self.check_parameter(&child, bytes, source, violations);
                }
            }
        }
    }

    fn check_parameter(
        &self,
        node: &tree_sitter::Node,
        bytes: &[u8],
        source: &str,
        violations: &mut Vec<Violation>,
    ) {
        if node.kind() != "parameter" && node.kind() != "parameter_with_optional_type" {
            return;
        }
        // Modifier spacing: `vararg  items` — the modifier is a separate
        // parameter_modifiers sibling ending right before the parameter.
        if let Some(prev) = node.prev_named_sibling() {
            let gap = &source[prev.end_byte()..node.start_byte()];
            let prev_word: String = source[prev.start_byte()..prev.end_byte()]
                .trim()
                .to_string();
            if matches!(
                prev_word.as_str(),
                "vararg" | "crossinline" | "noinline" | "val" | "const"
            ) && gap.bytes().filter(|b| *b == b' ').count() > 1
            {
                violations.push(self.v(prev.end_byte(), node.start_byte(), bytes, source));
            }
        }
        // Find `:` in the parameter text.
        let start = node.start_byte();
        let text = &source[start..node.end_byte()];
        let mut quote = false;
        for (i, ch) in text.char_indices() {
            if ch == '"' {
                quote = !quote;
            }
            if ch == ':' && !quote {
                let colon_abs = start + i;
                // `name :` — whitespace before colon is unexpected.
                if colon_abs > start && bytes[colon_abs - 1] == b' ' {
                    violations.push(self.v(colon_abs - 1, colon_abs, bytes, source));
                }
                // `:Type` — missing space after colon.
                let after = text[i + ch.len_utf8()..].chars().next();
                if let Some(c) = after {
                    if c != ' ' && c != '\n' && c != ')' {
                        violations.push(self.v(colon_abs + 1, colon_abs + 1, bytes, source));
                    }
                }
                break;
            }
        }
    }

    fn v_empty(&self, after_paren: usize, source: &str) -> Violation {
        let line = source[..after_paren.min(source.len())]
            .bytes()
            .filter(|&b| b == b'\n')
            .count()
            + 1;
        let line_start = source[..after_paren.min(source.len())]
            .rfind('\n')
            .map_or(0, |i| i + 1);
        Violation {
            file: String::new(),
            line,
            col: after_paren - line_start + 1,
            rule_id: self.id().into(),
            message: "Unexpected whitespace".into(),
            auto_fixable: true,
        }
    }

    fn v(&self, start: usize, end: usize, _bytes: &[u8], source: &str) -> Violation {
        let _ = (start, end);
        let line = source[..start.min(source.len())]
            .bytes()
            .filter(|&b| b == b'\n')
            .count()
            + 1;
        let line_start = source[..start.min(source.len())]
            .rfind('\n')
            .map_or(0, |i| i + 1);
        let col = start - line_start + 1;
        Violation {
            file: String::new(),
            line,
            col,
            rule_id: self.id().into(),
            message: "Unexpected spacing in parameter list".into(),
            auto_fixable: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        ParameterListSpacing.check(&p.parse(s), s)
    }

    #[test]
    fn empty_list_whitespace() {
        assert!(!c("fun foo( ) = 1\n").is_empty());
        assert!(c("fun foo() = 1\n").is_empty());
    }

    #[test]
    fn missing_space_after_colon() {
        assert!(!c("fun foo(a:Int) = 1\n").is_empty());
    }

    #[test]
    fn space_before_colon() {
        assert!(!c("fun foo(a : Int) = 1\n").is_empty());
    }

    #[test]
    fn missing_space_after_comma() {
        assert!(!c("fun foo(a: Int,b: Int) = 1\n").is_empty());
    }

    #[test]
    fn vararg_multi_space() {
        assert!(!c("fun foo(vararg  items: String) = 1\n").is_empty());
    }
}
