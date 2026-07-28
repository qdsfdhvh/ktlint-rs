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
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        // Only inspect whitespace between direct list children. Scanning the full node
        // span incorrectly reports intentional blank lines inside nested lambdas,
        // `when` blocks, and anonymous objects passed as arguments.
        for pair in children.windows(2) {
            let previous = pair[0];
            let next = pair[1];
            if previous.end_byte() >= next.start_byte() {
                continue;
            }
            let gap = &bytes[previous.end_byte()..next.start_byte()];
            let parts: Vec<_> = gap.split(|byte| *byte == b'\n').collect();
            // The final segment is indentation before `next`, not a blank line.
            for (index, line) in parts
                .iter()
                .enumerate()
                .skip(1)
                .take(parts.len().saturating_sub(2))
            {
                if line.iter().all(|byte| byte.is_ascii_whitespace()) {
                    violations.push(Violation {
                        file: String::new(),
                        line: previous.end_position().row + index + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Blank line inside list is not allowed".into(),
                        auto_fixable: true,
                    });
                }
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

    #[test]
    fn blank_lines_inside_nested_lambda_are_allowed() {
        let source = "call(\n    onAction = { action ->\n        when (action) {\n            A -> first()\n\n            B -> second()\n        }\n    },\n)";
        assert!(check(source).is_empty());
    }
}
