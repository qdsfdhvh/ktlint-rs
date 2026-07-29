//! `standard:modifier-list-spacing` — one separator after each modifier.

use crate::rules::{Rule, Violation};

pub struct ModifierListSpacing;

impl Rule for ModifierListSpacing {
    fn id(&self) -> &'static str {
        "standard:modifier-list-spacing"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        for (line_index, line) in source.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("context(") {
                if let Some(closing) = code.find(')') {
                    let rest = &code[closing + 1..];
                    if rest.starts_with(char::is_whitespace) && !rest.trim_start().is_empty() {
                        violations.push(Violation {
                            file: String::new(),
                            line: line_index + 1,
                            col: line.len() - code.len() + closing + 2,
                            rule_id: self.id().into(),
                            message: "Single newline expected after context receiver list".into(),
                            auto_fixable: true,
                        });
                    }
                }
            }
        }
        walk(tree.root_node(), &mut |node| {
            if node.kind() != "modifiers" {
                return;
            }
            let mut modifiers = modifier_children(node, source);
            if modifiers.is_empty() {
                return;
            }
            let declaration_start = node
                .next_sibling()
                .map_or(node.end_byte(), |next| next.start_byte());
            for index in 0..modifiers.len() {
                let modifier = modifiers[index];
                let next_start = modifiers
                    .get(index + 1)
                    .map_or(declaration_start, tree_sitter::Node::start_byte);
                if next_start < modifier.end_byte() || next_start > source.len() {
                    continue;
                }
                let gap = &source[modifier.end_byte()..next_start];
                let annotation = modifier.kind() == "annotation";
                let message = if annotation {
                    annotation_gap_message(gap)
                } else if gap != " " {
                    Some("Single whitespace expected after modifier")
                } else {
                    None
                };
                if let Some(message) = message {
                    let end = modifier.end_position();
                    violations.push(Violation {
                        file: String::new(),
                        line: end.row + 1,
                        col: end.column + 1,
                        rule_id: self.id().into(),
                        message: message.into(),
                        auto_fixable: true,
                    });
                }
            }
            modifiers.clear();
        });
        violations
    }
}

fn annotation_gap_message(gap: &str) -> Option<&'static str> {
    let newline_count = gap.bytes().filter(|byte| *byte == b'\n').count();
    if newline_count > 1 {
        Some("Single newline expected after annotation")
    } else if newline_count == 1 || gap == " " || contains_comment(gap) {
        None
    } else {
        Some("Single whitespace or newline expected after annotation")
    }
}

fn contains_comment(text: &str) -> bool {
    text.contains("//") || text.contains("/*")
}

fn modifier_children<'tree>(
    modifiers: tree_sitter::Node<'tree>,
    source: &str,
) -> Vec<tree_sitter::Node<'tree>> {
    let mut result = Vec::new();
    let bytes = source.as_bytes();
    for index in 0..modifiers.named_child_count() {
        let Some(child) = modifiers.named_child(index) else {
            continue;
        };
        if child.kind() == "annotation" && bytes.get(child.start_byte()) != Some(&b'@') {
            continue;
        }
        result.push(child);
    }
    result.sort_by_key(tree_sitter::Node::start_byte);
    result
}

fn walk(root: tree_sitter::Node, visit: &mut impl FnMut(tree_sitter::Node)) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        visit(node);
        for index in (0..node.child_count()).rev() {
            if let Some(child) = node.child(index) {
                stack.push(child);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let tree = KotlinParser::new().parse(source);
        ModifierListSpacing.check(&tree, source)
    }

    #[test]
    fn reports_redundant_spaces_after_modifiers() {
        let violations =
            check("abstract  class Foo {\n    protected  abstract  suspend  fun execute()\n}\n");
        let positions: Vec<_> = violations
            .iter()
            .map(|item| (item.line, item.col))
            .collect();
        assert_eq!(positions, vec![(1, 9), (2, 14), (2, 24), (2, 33)]);
        assert!(violations
            .iter()
            .all(|item| item.message == "Single whitespace expected after modifier"));
    }

    #[test]
    fn annotation_allows_one_space_or_one_newline() {
        assert!(check("@Foo1 @Foo2\nclass Bar\n").is_empty());
    }

    #[test]
    fn reports_multiple_newlines_after_annotations() {
        let violations = check("@Foo1\n\n@Foo2\n\nclass Bar\n");
        assert_eq!(violations.len(), 2);
        assert_eq!((violations[0].line, violations[0].col), (1, 6));
        assert_eq!((violations[1].line, violations[1].col), (3, 6));
        assert!(violations
            .iter()
            .all(|item| item.message == "Single newline expected after annotation"));
    }
}
