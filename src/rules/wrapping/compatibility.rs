//! ktlint 1.8 wrapping rules: expression-operand-wrapping, context-receiver
//! list wrapping, parameter-list-wrapping and the general `standard:wrapping`
//! block/argument newline checks.

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
            let trimmed_line = line.trim_start();
            if trimmed_line.starts_with("//")
                || trimmed_line.starts_with("/*")
                || trimmed_line.starts_with("*")
                || trimmed_line.starts_with("/**")
            {
                continue;
            }
            if let Some(operator_end) = unwrapped_operand_after_operator(line) {
                let operand = line[operator_end..]
                    .find(|character: char| !character.is_whitespace())
                    .map(|i| operator_end + i)
                    .unwrap_or(operator_end);
                violations.push(Violation {
                    file: String::new(),
                    line: line_index + 1,
                    col: operand + 1,
                    rule_id: self.id().to_string(),
                    message: "Newline expected before operand in multiline expression".into(),
                    auto_fixable: true,
                });
            }
        }
        violations
    }
}

/// `context(...) fun` — the declaration must start on a new line after the
/// context parameter list.
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

/// Wraps multiline function and constructor parameter lists: the first
/// parameter must not share the opening-paren line (issue #204).
pub struct ParameterListWrapping;

impl Rule for ParameterListWrapping {
    fn id(&self) -> &'static str {
        "standard:parameter-list-wrapping"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if matches!(
                node.kind(),
                "function_value_parameters" | "class_parameters"
            ) && node.start_position().row != node.end_position().row
            {
                let mut w = node.walk();
                let params: Vec<tree_sitter::Node> = node
                    .children(&mut w)
                    .filter(|c| matches!(c.kind(), "parameter" | "class_parameter"))
                    .collect();
                if let Some(first) = params.first() {
                    if first.start_position().row == node.start_position().row {
                        let pos = first.start_position();
                        violations.push(Violation {
                            file: String::new(),
                            line: pos.row + 1,
                            col: pos.column + 1,
                            rule_id: self.id().to_string(),
                            message: "Parameter should start on a newline".into(),
                            auto_fixable: true,
                        });
                    }
                }
                // A closing paren sharing the last parameter's line
                // (`beta: String)` — issue #204).
                if let Some(last) = params.last() {
                    if let Some(rp) = node.children(&mut node.walk()).find(|c| c.kind() == ")") {
                        if rp.start_position().row == last.end_position().row {
                            let pos = rp.start_position();
                            violations.push(Violation {
                                file: String::new(),
                                line: pos.row + 1,
                                col: pos.column + 1,
                                rule_id: self.id().to_string(),
                                message: "Missing newline before \")\"".into(),
                                auto_fixable: true,
                            });
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
        let _ = source;
        violations
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
