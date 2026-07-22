//! standard:no-multi-spaces — flag multiple consecutive spaces outside strings/comments.
use crate::rules::{Rule, Violation};

pub struct NoMultiSpaces;

impl Rule for NoMultiSpaces {
    fn id(&self) -> &'static str {
        "standard:no-multi-spaces"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        // Collect string and comment byte spans to mask them
        let mut masked = Vec::new();
        collect_masked_spans(tree.root_node(), &mut masked);

        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("  ") && !trimmed.starts_with("//") && !trimmed.starts_with('*') {
                let stripped = line.trim_start();
                if stripped.contains("  ") {
                    // Locate the byte position of the first double space
                    let line_off = source[..source.find(line).unwrap_or(0)].len();
                    let col = stripped.find("  ").unwrap_or(0) + (line.len() - stripped.len());
                    let pos = line_off + col;
                    if is_masked(pos, &masked) {
                        continue;
                    }
                    violations.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: self.id().to_string(),
                        message: "Multiple consecutive spaces".to_string(),
                        auto_fixable: true,
                    });
                }
            }
        }
        violations
    }
}

fn collect_masked_spans(node: tree_sitter::Node, spans: &mut Vec<(usize, usize)>) {
    let kind = node.kind();
    if matches!(
        kind,
        "string_literal" | "multiline_string_literal" | "line_comment" | "multiline_comment"
    ) {
        spans.push((node.start_byte(), node.end_byte()));
        return; // don't recurse into children of masked nodes
    }
    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) {
            collect_masked_spans(c, spans);
        }
    }
}

fn is_masked(pos: usize, spans: &[(usize, usize)]) -> bool {
    spans.iter().any(|(s, e)| pos >= *s && pos < *e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        NoMultiSpaces.check(&p.parse(s), s)
    }
    #[test]
    fn single_space() {
        assert!(check("val x = 1\n").is_empty());
    }
    #[test]
    fn multi_space() {
        let v = check("val x  = 1\n");
        assert!(!v.is_empty());
    }
    #[test]
    fn indentation_ok() {
        assert!(check("    val x = 1\n").is_empty());
    }
    #[test]
    fn issue71_no_flag_inside_string() {
        // Double spaces inside a string literal must NOT be flagged.
        assert!(check("val s = \"   \"\n").is_empty());
    }
    #[test]
    fn issue71_no_flag_inside_comment() {
        assert!(check("//    comment\nval x = 1\n").is_empty());
    }
}
