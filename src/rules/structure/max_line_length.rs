//! standard:max-line-length — lines should not exceed the configured limit
//! (default 120). Mirrors ktlint 1.8:
//! - Raw/multiline string interiors (triple-quoted blocks) are data, not code —
//!   not measured.
//! - Ordinary lines containing string literals ARE measured (`val a = "<long>"`).
//! - package/import lines are exempt (ktlint does not report them).

use crate::rules::{Rule, Violation};

pub struct MaxLineLength {
    max_length: usize,
}

impl MaxLineLength {
    pub fn new(max_length: usize) -> Self {
        let max_length = if max_length == 0 { 120 } else { max_length };
        Self { max_length }
    }
}

impl Rule for MaxLineLength {
    fn id(&self) -> &'static str {
        "standard:max-line-length"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let max_length = self.max_length;

        // Rows covered by a multiline (raw) string literal are data content —
        // not measured.
        let raw_string_rows = raw_string_rows(tree.root_node());
        // Rows holding an over-long string literal segment: ktlint only
        // reports a long line when it contains a string literal longer than
        // the limit — a long code line that wrapping rules can repair is
        // never reported (oracle: 140-char `= delegate.dispatch(…)` bodies
        // stay untouched; `val x = "aaa…"` reports).
        let long_string_rows = long_string_rows(tree.root_node(), source, max_length);

        source
            .lines()
            .enumerate()
            .filter(|(i, line)| {
                if raw_string_rows.contains(i) {
                    return false;
                }
                let trimmed = line.trim_start();
                // ktlint exempts a line holding only a single string template
                // (and optionally a trailing comma) — e.g. long string
                // literals, base64/SVG payloads. Verified against ktlint 1.8.
                let only_string = {
                    let body = trimmed.trim_end_matches(',');
                    body.starts_with('"') && body.ends_with('"') && body.len() > 2
                };
                line.chars().count() > max_length
                    && !only_string
                    && long_string_rows.contains(i)
                    && !trimmed.starts_with("package ")
                    && !trimmed.starts_with("import ")
                    && !trimmed.starts_with("//")
                    && !trimmed.starts_with("/*")
                    && !trimmed.starts_with('*')
            })
            .map(|(i, _line)| Violation {
                file: String::new(),
                line: i + 1,
                col: max_length,
                rule_id: self.id().to_string(),
                message: format!("Exceeded max line length ({})", max_length),
                auto_fixable: false, // wrapping requires manual intervention
            })
            .collect()
    }
}

/// Rows containing a string literal longer than the limit on that row. Only
/// these rows can trigger max-line-length — a long pure-code line is
/// reported by the wrapping rules instead (JVM oracle).
fn long_string_rows(
    node: tree_sitter::Node,
    source: &str,
    max_length: usize,
) -> std::collections::HashSet<usize> {
    let mut rows = std::collections::HashSet::new();
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if current.kind() == "string_literal" && current.end_position().row == current.start_position().row {
            let start = current.start_byte();
            let end = current.end_byte().min(source.len());
            if source[start..end].chars().count() > max_length {
                rows.insert(current.start_position().row);
            }
        }
        for i in (0..current.child_count()).rev() {
            if let Some(child) = current.child(i) {
                stack.push(child);
            }
        }
    }
    rows
}

/// Rows covered by a multiline string literal (`""" ... """`).
fn raw_string_rows(node: tree_sitter::Node) -> std::collections::HashSet<usize> {
    let mut rows = std::collections::HashSet::new();
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if current.kind() == "string_literal"
            && current.end_position().row > current.start_position().row
        {
            for row in current.start_position().row..=current.end_position().row {
                rows.insert(row);
            }
        }
        for i in (0..current.child_count()).rev() {
            if let Some(child) = current.child(i) {
                stack.push(child);
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        MaxLineLength::new(120).check(&tree, source)
    }

    #[test]
    fn raw_string_interior_not_measured() {
        let long = "あ".repeat(130);
        let src = format!("val doc: String = \"\"\"\n    {long}\n\"\"\".trimIndent()\n");
        assert!(
            check(&src).is_empty(),
            "raw string interior must not be measured"
        );
    }

    #[test]
    fn ordinary_long_line_measured() {
        let long = "x".repeat(130);
        let src = format!("val a: String = \"{long}\"\n");
        assert!(
            !check(&src).is_empty(),
            "long line with string must be reported"
        );
    }

    #[test]
    fn short_line_ok() {
        assert!(check("val a = 1\n").is_empty());
    }
}
