//! Unimplemented ktlint-standard rules that exist as test files but not in ktlint-rs.
//! This file tracks our gap to full ktlint parity.
//!
//! Gap: 59 unmapped test files → ~40 missing rules
//! Current: 62 rules implemented
//! Target: 100+ rules for full parity

use crate::rules::{Rule, Violation};

// ── Basic spacing rules (low effort, high impact) ──

pub struct SpacingAroundDot;
impl Rule for SpacingAroundDot {
    fn id(&self) -> &'static str {
        "standard:dot-spacing"
    }

    fn auto_fixable(&self) -> bool {
        true
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut seen = std::collections::HashSet::new();
        collect_dots(tree.root_node(), source, &mut violations, &mut seen);
        violations
    }
}

/// Collect real dot operators (navigation) from the CST, skipping comments and
/// string literals, and ignoring line-leading dots (chained calls).
fn collect_dots(
    node: tree_sitter::Node,
    source: &str,
    out: &mut Vec<Violation>,
    seen: &mut std::collections::HashSet<usize>,
) {
    if node.kind() == "." {
        let start = node.start_byte();
        if seen.insert(start) {
            check_dot(node, source, out);
        }
        return;
    }
    if node.kind().contains("comment") || node.kind().contains("string") {
        return;
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_dots(child, source, out, seen);
        }
    }
}

fn check_dot(node: tree_sitter::Node, source: &str, out: &mut Vec<Violation>) {
    let start = node.start_byte();
    let end = node.end_byte();
    let before = source[..start].chars().last();
    let after = source[end..].chars().next();
    // Line-leading dot (only whitespace since the last newline before it) is a
    // chained call continuation — valid.
    let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
    let before_ws_only = source[line_start..start].trim().is_empty();
    let space_before = before == Some(' ') && !before_ws_only;
    let space_after = after == Some(' ') || after == Some('\t');
    let line_col = start - line_start + 1;
    // ktlint reports the offending whitespace column: the space before the dot
    // (line_col - 1) or the space after it (line_col + 1).
    let (col, message) = match (space_before, space_after) {
        (true, true) => (line_col - 1, "Unexpected spacing around \".\""),
        (true, false) => (line_col - 1, "Unexpected spacing before \".\""),
        (false, true) => (line_col + 1, "Unexpected spacing after \".\""),
        (false, false) => return,
    };
    let line = source[..start].bytes().filter(|&b| b == b'\n').count() + 1;
    out.push(Violation {
        file: String::new(),
        line,
        col,
        rule_id: "standard:dot-spacing".into(),
        message: message.into(),
        auto_fixable: true,
    });
}

pub struct SpacingAroundSquareBrackets;
impl Rule for SpacingAroundSquareBrackets {
    fn id(&self) -> &'static str {
        "standard:square-brackets-spacing"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            let t = l.trim();
            if t.contains("[ ") || t.contains(" ]") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Unexpected space inside square brackets".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct NoBlankLinesInChainedMethodCalls;
impl Rule for NoBlankLinesInChainedMethodCalls {
    fn id(&self) -> &'static str {
        "standard:no-blank-lines-in-chained-method-calls"
    }
    /// Mirrors ktlint 1.8: a blank line between two chained method calls
    /// (foo() then a blank line then .bar()) is needless. Reported at the continuation line.
    fn check(&self, _t: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        let mut in_block_comment = false;
        let mut in_triple_string = false;
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            // Multiline strings (`""" … """`): interior lines (including
            // blank ones) are content, never chain separators.
            if in_triple_string {
                if t.contains("\"\"\"") {
                    in_triple_string = false;
                }
                continue;
            }
            if t.contains("\"\"\"") {
                let only = t.trim_end() == "\"\"\"";
                let starts = t.starts_with("\"\"\"");
                let ends = t.trim_end().ends_with("\"\"\"");
                if only || starts != ends {
                    in_triple_string = !in_triple_string;
                }
                continue;
            }
            // Track `/* … */` block comments: interior lines (including blank
            // ones) are comment content, never chain separators.
            if in_block_comment {
                if t.contains("*/") {
                    in_block_comment = false;
                }
                continue;
            }
            if t.starts_with("/*") {
                if !t.contains("*/") {
                    in_block_comment = true;
                }
                continue;
            }
            if t.starts_with("//") || t.starts_with('*') {
                continue;
            }
            if !t.is_empty() {
                continue;
            }
            // Skip continuation blank lines of a run (report once, at the
            // first blank line — ktlint reports the whitespace start + 1).
            if lines[..i]
                .iter()
                .next_back()
                .is_some_and(|l| l.trim().is_empty())
            {
                continue;
            }
            // Blank line: check the previous and next non-blank lines for a
            // chain (`… .foo()` / `.foo()`), ignoring comment lines.
            let prev = lines[..i].iter().rev().find(|l| !l.trim().is_empty());
            let next = lines[i + 1..].iter().find(|l| !l.trim().is_empty());
            let prev_chain = prev.is_some_and(|l| {
                let lt = l.trim();
                !is_comment_line(lt) && lt.ends_with('.')
            });
            let next_chain = next.is_some_and(|l| {
                let lt = l.trim_start();
                !is_comment_line(lt) && lt.starts_with('.')
            });
            if prev_chain || next_chain {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Needless blank line(s)".into(),
                    auto_fixable: true,
                });
            }
        }
        violations
    }
}

/// True for comment/KDoc lines (`//`, `/* … */`, `* …` KDoc body lines).
fn is_comment_line(t: &str) -> bool {
    t.starts_with("//") || t.starts_with("/*") || t.starts_with('*')
}

pub struct NoLineBreakAfterElse;
impl Rule for NoLineBreakAfterElse {
    fn id(&self) -> &'static str {
        "standard:no-line-break-after-else"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            if ln.trim() == "else" && i + 1 < l.len() && l[i + 1].trim().is_empty() {
                v.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Unexpected blank line after else".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct NoLineBreakBeforeAssignment;
impl Rule for NoLineBreakBeforeAssignment {
    fn id(&self) -> &'static str {
        "standard:no-line-break-before-assignment"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            if ln.trim().starts_with('=') && i > 0 && !l[i - 1].trim().is_empty() {
                v.push(Violation {
                    file: String::new(),
                    line: i,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Line break before \"=\" should be avoided".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct NoConsecutiveComments;
impl Rule for NoConsecutiveComments {
    fn id(&self) -> &'static str {
        "standard:no-consecutive-comments"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for i in 0..l.len().saturating_sub(1) {
            if l[i].trim().starts_with("//") && l[i + 1].trim().starts_with("//") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Consecutive comments should be combined".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct NullableTypeSpacing;
impl Rule for NullableTypeSpacing {
    fn id(&self) -> &'static str {
        "standard:nullable-type-spacing"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "nullable_type" {
                let text = &source[node.byte_range()];
                if let Some(question) = text.rfind('?') {
                    let mut whitespace = question;
                    while whitespace > 0 && text.as_bytes()[whitespace - 1].is_ascii_whitespace() {
                        whitespace -= 1;
                    }
                    if whitespace < question {
                        let offset = node.start_byte() + whitespace;
                        let before = &source[..offset];
                        let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
                        let line_start = before.rfind('\n').map_or(0, |index| index + 1);
                        violations.push(Violation {
                            file: String::new(),
                            line,
                            col: source[line_start..offset].chars().count() + 1,
                            rule_id: self.id().into(),
                            message: "Unexpected whitespace".into(),
                            auto_fixable: true,
                        });
                    }
                }
            }
            for index in (0..node.child_count()).rev() {
                if let Some(child) = node.child(index) {
                    stack.push(child);
                }
            }
        }
        violations
    }
}

#[cfg(test)]
mod nullable_type_spacing_tests {
    use super::*;
    use crate::parser::KotlinParser;

    #[test]
    fn reports_whitespace_before_nullable_marker_with_ktlint_message() {
        let source = "fun String ?.normalized() = trim()\n";
        let tree = KotlinParser::new().parse(source);
        let violations = NullableTypeSpacing.check(&tree, source);
        assert_eq!(violations.len(), 1);
        assert_eq!((violations[0].line, violations[0].col), (1, 11));
        assert_eq!(violations[0].message, "Unexpected whitespace");
        assert!(violations[0].auto_fixable);
    }

    #[test]
    fn accepts_nullable_type_without_whitespace() {
        let source = "fun String?.normalized() = trim()\n";
        let tree = KotlinParser::new().parse(source);
        assert!(NullableTypeSpacing.check(&tree, source).is_empty());
    }
}
