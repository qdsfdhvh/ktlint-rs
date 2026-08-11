//! standard:wrapping — general wrapping rule for when/if/for/while expressions.
//! Enforces that in multiline expressions, continuation elements are on new lines.
use crate::rules::{Rule, Violation};

pub struct GeneralWrapping;

impl Rule for GeneralWrapping {
    fn id(&self) -> &'static str {
        "standard:wrapping"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        // Issue #204: a function/class body's `{` must be followed by a
        // newline, and a multiline call/parameter list's `(` must not share
        // a line with its first argument.
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "{" {
                let block = node
                    .parent()
                    .is_some_and(|p| matches!(p.kind(), "function_body" | "class_body"));
                if block {
                    report_after(node, bytes, source, '{', &mut violations);
                }
            } else if node.kind() == "(" || node.kind() == ")" {
                let multiline = node.parent().is_some_and(|p| {
                    matches!(p.kind(), "value_arguments" | "function_value_parameters")
                        && p.start_position().row != p.end_position().row
                });
                if multiline {
                    if node.kind() == "(" {
                        report_after(node, bytes, source, '(', &mut violations);
                    } else {
                        // `"b")` — a closing paren sharing the last
                        // argument's line (issue #204).
                        let start = node.start_byte();
                        if start > 0 && bytes[start - 1] != b'\n' {
                            let prev_nonws = bytes[..start]
                                .iter()
                                .rposition(|&b| b != b' ' && b != b'\t' && b != b'\n');
                            let last_arg = node.parent().is_some_and(|p| {
                                p.children(&mut p.walk()).any(|c| {
                                    matches!(c.kind(), "value_argument" | "parameter")
                                        && c.end_position().row == node.start_position().row
                                })
                            });
                            if last_arg {
                                // oracle reports at the char before `)`
                                // (`beta: String)` -> 4:16).
                                let col = prev_nonws
                                    .map(|i| {
                                        let line_start = bytes[..i]
                                            .iter()
                                            .rposition(|&b| b == b'\n')
                                            .map_or(0, |j| j + 1);
                                        i - line_start + 1
                                    })
                                    .unwrap_or(node.start_position().column + 1);
                                violations.push(Violation {
                                    file: String::new(),
                                    line: node.start_position().row + 1,
                                    col,
                                    rule_id: self.id().into(),
                                    message: "Missing newline before \")\"".into(),
                                    auto_fixable: true,
                                });
                            }
                        }
                    }
                }
            }
            for i in (0..node.child_count()).rev() {
                if let Some(c) = node.child(i) {
                    stack.push(c);
                }
            }
        }
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            // Check `when` entries — `1, 2 ->` on same line is wrong for multiline when
            if trimmed.starts_with("when ") && i + 1 < lines.len() {
                let mut prev_line_was_entry = false;
                for t in lines[i + 1..]
                    .iter()
                    .take(lines.len().min(i + 50).saturating_sub(i + 1))
                    .map(|l| l.trim())
                {
                    if t == "}" || t == "})" || t == ")." {
                        break;
                    }
                    if t.contains("->") && !t.contains("\"") {
                        if prev_line_was_entry {
                            // Multiple entries on consecutive lines — check consistency
                        }
                        prev_line_was_entry = true;
                    } else if !t.is_empty() && !t.starts_with("//") {
                        prev_line_was_entry = false;
                    }
                }
            }

            // Check `if/else` chain consistency
            if (trimmed.starts_with("if (")
                || trimmed.starts_with("for (")
                || trimmed.starts_with("while ("))
                && trimmed.ends_with('{')
                && i + 1 < lines.len()
                && lines[i + 1].trim().is_empty()
            {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().to_string(),
                    message: "Unexpected blank line after if-condition".to_string(),
                    auto_fixable: true,
                });
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        GeneralWrapping.check(&p.parse(s), s)
    }
    #[test]
    fn if_no_blank() {
        assert!(check("if (x) {\n    doA()\n}\n").is_empty());
    }
    #[test]
    fn if_with_blank() {
        let v = check("if (x) {\n\n    doA()\n}\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:wrapping");
    }

    #[test]
    fn loop_with_blank() {
        assert!(!check("for (x in xs) {\n\n    use(x)\n}\n").is_empty());
        assert!(!check("while (ready) {\n\n    tick()\n}\n").is_empty());
    }
}

/// Report when a non-whitespace char follows the delimiter on the same line.
fn report_after(
    node: tree_sitter::Node,
    bytes: &[u8],
    source: &str,
    delim: char,
    violations: &mut Vec<Violation>,
) {
    let start = node.end_byte();
    let line_end = bytes[start..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |i| start + i);
    if let Some(off) = bytes[start..line_end]
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
    {
        let pos = start + off;
        // An empty block `{}` is fine on one line.
        if delim == '{' && bytes.get(pos) == Some(&b'}') {
            return;
        }
        // Oracle reports at the delimiter itself: `{ x` → the `{` column.
        let line = source[..start].bytes().filter(|&b| b == b'\n').count() + 1;
        let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
        violations.push(Violation {
            file: String::new(),
            line,
            col: start - line_start + 1,
            rule_id: "standard:wrapping".into(),
            message: format!("Missing newline after \"{delim}\""),
            auto_fixable: true,
        });
    }
}
