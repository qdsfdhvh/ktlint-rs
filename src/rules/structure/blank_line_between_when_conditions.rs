//! standard:blank-line-between-when-conditions
//!
//! ktlint 1.8: when at least one when-condition is multiline (or a branch is
//! preceded by a comment), a blank line is required between every
//! when-condition. Otherwise branches sit adjacent.
use crate::rules::{Rule, Violation};

pub struct BlankLineBetweenWhenConditions;

impl Rule for BlankLineBetweenWhenConditions {
    fn id(&self) -> &'static str {
        "standard:blank-line-between-when-conditions"
    }
    fn check(&self, tree: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "when_expression" {
                // Direct when-entry children (skip nested whens inside bodies).
                let entries: Vec<tree_sitter::Node> = {
                    let mut w = node.walk();
                    node.children(&mut w)
                        .filter(|c| c.kind() == "when_entry")
                        .collect()
                };
                if entries.len() < 2 {
                    let mut w = node.walk();
                    for c in node.children(&mut w) {
                        stack.push(c);
                    }
                    continue;
                }
                // A condition is multiline when the whole when-entry spans
                // more than one line (a block body `1 -> {\n ... \n}` counts,
                // matching ktlint 1.8). Single-line entries — including a
                // single-line block `{ println() }` — do not.
                let has_multiline = entries
                    .iter()
                    .any(|e| e.start_position().row != e.end_position().row);
                if has_multiline {
                    for k in 1..entries.len() {
                        let prev = entries[k - 1];
                        let cur = entries[k];
                        // Whitespace between the previous entry's end and this
                        // entry's start must contain a blank line.
                        let gap = &s[prev.end_byte()..cur.start_byte()];
                        let blank_count = gap.matches('\n').count();
                        if blank_count < 2 {
                            let line = s[..cur.start_byte()]
                                .bytes()
                                .filter(|&b| b == b'\n')
                                .count()
                                + 1;
                            v.push(Violation {
                                file: String::new(),
                                line,
                                col: 1,
                                rule_id: self.id().into(),
                                message:
                                    "Add a blank line between all when-conditions in case at least one multiline when-condition is found in the statement"
                                        .into(),
                                auto_fixable: true,
                            });
                        }
                    }
                }
            }
            let mut w = node.walk();
            for c in node.children(&mut w) {
                stack.push(c);
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(src);
        BlankLineBetweenWhenConditions.check(&tree, src)
    }

    #[test]
    fn all_single_line_entries_no_violation() {
        let src = "fun f(x: Int) {\n    when (x) {\n        1 -> println(\"one\")\n        2 -> println(\"two\")\n        else -> println(\"other\")\n    }\n}\n";
        assert!(check(src).is_empty());
    }

    #[test]
    fn one_multiline_entry_separates_all() {
        let src = "fun f(x: Int) {\n    when (x) {\n        1 -> {\n            println(\"one\")\n            println(\"uno\")\n        }\n        2 -> println(\"two\")\n        else -> println(\"other\")\n    }\n}\n";
        let v = check(src);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].line, 7); // `2 ->` line
        assert_eq!(v[1].line, 8); // `else ->` line
        assert_eq!(v[0].rule_id, "standard:blank-line-between-when-conditions");
    }

    #[test]
    fn single_line_block_body_is_not_multiline() {
        let src = "fun f(x: Int) {\n    when (x) {\n        1 -> { println(\"one\") }\n        2 -> println(\"two\")\n    }\n}\n";
        assert!(check(src).is_empty());
    }

    #[test]
    fn all_multiline_entries_still_separated() {
        let src = "fun f(x: Int) {\n    when (x) {\n        1 -> {\n            println(\"one\")\n        }\n        2 -> {\n            println(\"two\")\n        }\n    }\n}\n";
        let v = check(src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 6); // `2 -> {` line
    }

    #[test]
    fn multiline_condition_triggers() {
        let src = "fun f(x: Int) {\n    when (x) {\n        x +\n            1 -> println(\"one\")\n        2 -> println(\"two\")\n    }\n}\n";
        let v = check(src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 5); // `2 ->` line
    }
}
