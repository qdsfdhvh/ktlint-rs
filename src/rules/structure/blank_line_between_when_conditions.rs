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
                // A condition is multiline when a when-entry spans more than
                // one line (a block body `1 -> {\n ... \n}` counts, matching
                // ktlint 1.8). Single-line entries — including a single-line
                // block `{ println() }` — do not.
                //
                // Detection uses only each entry's *start row*, never its end
                // row or byte range: tree-sitter-kotlin mis-parses conditions
                // that start with a parenthesised comparison (`(a ?: 0) > 0 ->
                // ...` followed by another condition) and swallows the next
                // condition into the current entry's end, which would make a
                // clean single-line `when` look multiline (issue #160).
                let mut starts: Vec<usize> =
                    entries.iter().map(|e| e.start_position().row).collect();
                starts.sort_unstable();
                starts.dedup();
                let when_end = node.end_position().row;
                let has_multiline = starts.windows(2).any(|w| w[1] - w[0] > 1)
                    || starts.last().is_some_and(|&last| when_end - last > 1);
                if has_multiline {
                    // A blank line must separate each entry from the previous
                    // one. `starts` are the entries' start rows (reliable even
                    // when tree-sitter mis-parses the entry range); entry k is
                    // missing its blank line when the text between the end of
                    // entry k-1 and the start of entry k contains fewer than
                    // two newlines.
                    for k in 1..entries.len() {
                        let prev = entries[k - 1];
                        let cur = entries[k];
                        let gap = &s[prev.end_byte()..cur.start_byte()];
                        let blank_count = gap.matches('\n').count();
                        if blank_count < 2 {
                            // Report on the entry's own start row (start rows
                            // are reliable; the byte range is not).
                            let line = cur.start_position().row + 1;
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
    fn paren_conditions_all_single_line_no_violation() {
        // Issue #160 regression: `(a ?: 0) > 0 ->` conditions make
        // tree-sitter inflate the first entry's range into the next
        // condition, which used to look multiline. Detection must use entry
        // start rows only.
        let src = "class C {\n    fun f() {\n        val s = when {\n            (a ?: 0) > 0 -> \"A\"\n            (b ?: 0) > 0 -> \"B\"\n            else -> \"C\"\n        }\n    }\n}\n";
        assert!(check(src).is_empty());
    }

    #[test]
    fn multiline_condition_triggers() {
        let src = "fun f(x: Int) {\n    when (x) {\n        x +\n            1 -> println(\"one\")\n        2 -> println(\"two\")\n    }\n}\n";
        let v = check(src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 5); // `2 ->` line
    }
}
