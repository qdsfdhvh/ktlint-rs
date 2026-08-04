//! `standard:property-wrapping` — wrap a property declaration when it exceeds
//! the max line length. Mirrors ktlint 1.8.0 (PropertyWrappingRule):
//! when `baseIndent + prefix-length` exceeds max_line_length, insert a newline
//! before/after the first structural token that would split the line:
//! - before the type reference (`val x: <newline> Type`)
//! - after the colon (`val x: Type = <newline> value`)
//! - before the call expression (`val x: Type = <newline> foo(...)`)

use crate::rules::{Rule, Violation};

pub struct PropertyWrapping;

impl Rule for PropertyWrapping {
    fn id(&self) -> &'static str {
        "standard:property-wrapping"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "property_declaration" {
                self.check_property(&node, source, &mut violations);
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

impl PropertyWrapping {
    fn check_property(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        violations: &mut Vec<Violation>,
    ) {
        let max_line_length = 120; // default; configurable via .editorconfig
        let start = node.start_byte();
        // Base indent: the property's line indentation.
        let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
        let indent_len = start - line_start;
        if indent_len > max_line_length {
            return;
        }

        // If the property already spans multiple lines, nothing to wrap.
        if node.end_position().row > node.start_position().row {
            return;
        }

        let text = &source[start..node.end_byte()];
        // Prefix length from declaration start (after indent) up to each token.
        let prefix_len =
            |token: &str| -> Option<usize> { text.find(token).map(|i| indent_len + i) };

        // 1. Before the type reference: `val x: <newline> Type` — split after
        //    the colon if `val x:` already exceeds the limit.
        if let Some(colon) = prefix_len(":") {
            if colon > max_line_length {
                violations.push(self.v_at(start + text.find(':').unwrap(), source));
                return;
            }
        }

        // 2. After `=`: `val x: Type = <newline> value` — split when the
        //    `val x: Type =` prefix exceeds the limit.
        if let Some(eq) = prefix_len("=") {
            if eq > max_line_length {
                violations.push(self.v_at(start + text.find('=').unwrap(), source));
                return;
            }
        }

        // 3. Before the call expression: `val x: Type = <newline> foo(...)` —
        //    when the whole line exceeds the limit and there is a call.
        if text.contains('(') && text.contains(')') {
            let line_len = source[line_start..node.end_byte()].chars().count();
            if line_len > max_line_length {
                // Report at the first `(` of the call (wrap before it).
                if let Some(open) = text.find('(') {
                    violations.push(self.v_at(start + open, source));
                }
            }
        }
    }

    fn v_at(&self, offset: usize, source: &str) -> Violation {
        let line = source[..offset].bytes().filter(|&b| b == b'\n').count() + 1;
        let line_start = source[..offset].rfind('\n').map_or(0, |i| i + 1);
        let col = offset - line_start + 1;
        Violation {
            file: String::new(),
            line,
            col,
            rule_id: self.id().into(),
            message: "Missing newline before value".into(),
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
        PropertyWrapping.check(&p.parse(s), s)
    }

    #[test]
    fn short_property_ok() {
        assert!(c("val x: String = \"value\"\n").is_empty());
    }

    #[test]
    fn long_property_wraps() {
        let src = "val someRatherLongPropertyNameHere: List<String> = listOf(\"aaaaaaaaaa\", \"bbbbbbbbbb\", \"cccccccccc\", \"dddddddddd\", \"eeeeeeeeee\")\n";
        assert!(!c(src).is_empty(), "long property should report wrapping");
    }

    #[test]
    fn multiline_property_ok() {
        let src = "val x: String =\n    \"value\"\n";
        assert!(c(src).is_empty());
    }
}
