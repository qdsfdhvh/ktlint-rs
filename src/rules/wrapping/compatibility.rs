//! Deprecated ktlint 1.8 wrapping providers retained as no-op compatibility IDs.

use crate::rules::{Rule, Violation};

/// Deprecated since ktlint 1.7. All behavior moved to
/// `standard:expression-operand-wrapping`.
pub struct ConditionWrapping;

impl Rule for ConditionWrapping {
    fn id(&self) -> &'static str {
        "standard:condition-wrapping"
    }

    fn check(&self, _tree: &tree_sitter::Tree, _source: &str) -> Vec<Violation> {
        Vec::new()
    }
}

/// Wraps operands consistently when a binary expression spans multiple lines.
pub struct ExpressionOperandWrapping;

impl Rule for ExpressionOperandWrapping {
    fn id(&self) -> &'static str {
        "standard:expression-operand-wrapping"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        for (line_index, line) in source.lines().enumerate() {
            if let Some(operator_end) = unwrapped_operand_after_operator(line) {
                let operand = line[operator_end..]
                    .find(|character: char| !character.is_whitespace())
                    .map_or(operator_end, |offset| operator_end + offset);
                violations.push(Violation {
                    file: String::new(),
                    line: line_index + 1,
                    col: operand + 1,
                    rule_id: self.id().to_string(),
                    message: "Newline expected before operand in multiline expression".to_string(),
                    auto_fixable: true,
                });
            }
        }
        violations
    }
}

/// Enforces a line break between a context parameter list and its declaration.
pub struct ContextReceiverListWrapping;

impl Rule for ContextReceiverListWrapping {
    fn id(&self) -> &'static str {
        "standard:context-receiver-list-wrapping"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(line_index, line)| {
                let operand = context_declaration_on_same_line(line)?;
                Some(Violation {
                    file: String::new(),
                    line: line_index + 1,
                    col: operand + 1,
                    rule_id: self.id().to_string(),
                    message: "Expected a newline after the context parameter".to_string(),
                    auto_fixable: true,
                })
            })
            .collect()
    }
}

pub(crate) fn context_declaration_on_same_line(line: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start().len();
    let code = line.trim_start();
    if !code.starts_with("context(") {
        return None;
    }
    let closing = code.find(')')?;
    let rest = &code[closing + 1..];
    if rest.starts_with(char::is_whitespace) && !rest.trim_start().is_empty() {
        Some(indent + closing + 1 + (rest.len() - rest.trim_start().len()))
    } else {
        None
    }
}

pub(crate) fn unwrapped_operand_after_operator(line: &str) -> Option<usize> {
    let code = line.trim_end();
    let operators = top_level_wrappable_operators(code);
    if operators.len() < 2 {
        return None;
    }
    let &(last_start, last_len) = operators.last()?;
    if last_start + last_len != code.len() {
        return None;
    }
    let &(previous_start, previous_len) = operators.get(operators.len() - 2)?;
    let after_previous = previous_start + previous_len;
    (!code[after_previous..last_start].trim().is_empty()).then_some(after_previous)
}

fn top_level_wrappable_operators(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut operators = Vec::new();
    let mut depth = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b'"' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'"' {
                    index += if bytes[index] == b'\\' { 2 } else { 1 };
                }
            }
            b'&' | b'|' if depth == 0 && bytes.get(index + 1) == Some(&bytes[index]) => {
                operators.push((index, 2));
                index += 1;
            }
            b'+' | b'-' | b'*' | b'/' if depth == 0 => operators.push((index, 1)),
            _ => {}
        }
        index += 1;
    }
    operators
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    #[test]
    fn condition_wrapping_is_a_noop_in_ktlint_1_8() {
        let source = "if (first &&\n    second) {\n    Unit\n}\n";
        let tree = KotlinParser::new().parse(source);
        assert!(ConditionWrapping.check(&tree, source).is_empty());
    }

    #[test]
    fn finds_unwrapped_logical_operand() {
        let source = "val result =\n    first || second ||\n        third\n";
        let tree = KotlinParser::new().parse(source);
        let violations = ExpressionOperandWrapping.check(&tree, source);
        assert_eq!(violations.len(), 1);
        assert_eq!((violations[0].line, violations[0].col), (2, 14));
    }

    #[test]
    fn ignores_nested_single_line_expression() {
        let source = "val result =\n    (first + second) * third *\n        fourth\n";
        let tree = KotlinParser::new().parse(source);
        let violations = ExpressionOperandWrapping.check(&tree, source);
        assert_eq!(violations.len(), 1);
        assert_eq!((violations[0].line, violations[0].col), (2, 24));
    }

    #[test]
    fn finds_context_declaration_on_same_line() {
        let source = "context(_: Foo) fun example() = Unit\n";
        let tree = KotlinParser::new().parse(source);
        let violations = ContextReceiverListWrapping.check(&tree, source);
        assert_eq!(violations.len(), 1);
        assert_eq!((violations[0].line, violations[0].col), (1, 17));
    }

    #[test]
    fn ignores_wrapped_context_parameter() {
        let source = "context(_: Foo)\nfun example() = Unit\n";
        let tree = KotlinParser::new().parse(source);
        assert!(ContextReceiverListWrapping.check(&tree, source).is_empty());
    }
}
