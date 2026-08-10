//! standard:spacing-between-function-name-and-parenthesis — no space before (.
use crate::rules::{Rule, Violation};

pub struct FunctionNameParenSpacing;

impl Rule for FunctionNameParenSpacing {
    fn id(&self) -> &'static str {
        "standard:spacing-between-function-name-and-opening-parenthesis"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "function_declaration" {
                let mut w = node.walk();
                let kids: Vec<tree_sitter::Node> = node.children(&mut w).collect();
                // `fun name (` — the name (simple_identifier) must sit
                // directly against the parameter list's `(`. Works with any
                // leading modifiers (`public fun name (`), which the old
                // line-scan missed (issue #203).
                if let Some(idx) = kids.iter().position(|c| c.kind() == "simple_identifier") {
                    if let Some(paren) = kids
                        .iter()
                        .skip(idx + 1)
                        .find(|c| c.kind() == "function_value_parameters")
                    {
                        let name_end = kids[idx].end_byte();
                        let paren_start = paren.start_byte();
                        let gap = &bytes[name_end..paren_start];
                        if gap.iter().any(|&b| b == b' ' || b == b'\t') {
                            let pos = paren.start_position();
                            violations.push(Violation {
                                file: String::new(),
                                line: pos.row + 1,
                                col: pos.column + 1,
                                rule_id: self.id().to_string(),
                                message: "Unexpected whitespace between function name and opening parenthesis"
                                    .to_string(),
                                auto_fixable: true,
                            });
                        }
                    }
                }
            }
            let mut w = node.walk();
            let kids: Vec<tree_sitter::Node> = node.children(&mut w).collect();
            for k in kids.into_iter().rev() {
                stack.push(k);
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
        let t = p.parse(s);
        FunctionNameParenSpacing.check(&t, s)
    }
    #[test]
    fn normal_fun() {
        assert!(check("fun foo()\n").is_empty());
    }
    #[test]
    fn space_before_paren() {
        let v = check("fun foo ()\n");
        assert!(!v.is_empty());
        assert_eq!(
            v[0].rule_id,
            "standard:spacing-between-function-name-and-opening-parenthesis"
        );
    }
}
