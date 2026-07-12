use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

/// Checks that parameter/argument/value lists don't contain blank lines.
pub struct NoBlankLineInList;

impl Rule for NoBlankLineInList {
    fn id(&self) -> &'static str {
        "standard:no-blank-line-in-list"
    }

    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl NoBlankLineInList {
    fn walk(&self, node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let kind = node.kind();
        // Check list-like nodes: function_value_parameters, class_parameters,
        // value_arguments, type_arguments, type_parameters
        let is_list = kind == "function_value_parameters"
            || kind == "class_parameters"
            || kind == "value_arguments"
            || kind == "type_arguments"
            || kind == "type_parameters";

        if is_list {
            self.check_list(&node, bytes, violations);
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, bytes, violations);
            }
        }
    }

    fn check_list(&self, node: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let start_row = node.start_position().row;
        let end_row = node.end_position().row;
        if start_row == end_row {
            return;
        }

        for row in (start_row + 1)..end_row {
            let line_start = bytes
                .iter()
                .enumerate()
                .filter(|(_, &b)| b == b'\n')
                .nth(row.saturating_sub(1))
                .map(|(i, _)| i + 1)
                .unwrap_or(0);
            let line_end = bytes
                .iter()
                .enumerate()
                .skip(line_start)
                .filter(|(_, &b)| b == b'\n')
                .map(|(i, _)| i)
                .next()
                .unwrap_or(bytes.len());
            let line = &bytes[line_start..line_end];
            if line.iter().all(|&b| b.is_ascii_whitespace()) {
                violations.push(Violation {
                    file: String::new(),
                    line: row + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Blank line inside list is not allowed".into(),
                    auto_fixable: true,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        NoBlankLineInList.check(&p.parse(src), src)
    }

    #[test]
    fn single_line_no_violation() {
        let v = check("fun f(a: Int, b: Int) {}");
        assert!(v.is_empty());
    }

    #[test]
    fn multiline_no_blank() {
        let v = check("fun f(\n    a: Int,\n    b: Int\n) {}");
        assert!(v.is_empty());
    }

    #[test]
    fn blank_line_in_params() {
        let v = check("fun f(\n    a: Int,\n\n    b: Int\n) {}");
        assert!(!v.is_empty());
    }
}
