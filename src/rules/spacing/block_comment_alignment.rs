//! `standard:block-comment-initial-star-alignment`.

use crate::rules::{Rule, Violation};

pub struct BlockCommentInitialStarAlignment;

impl Rule for BlockCommentInitialStarAlignment {
    fn id(&self) -> &'static str {
        "standard:block-comment-initial-star-alignment"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        walk(tree.root_node(), &mut |node| {
            if !matches!(node.kind(), "block_comment" | "multiline_comment") {
                return;
            }
            let expected = node.start_position().column + 1;
            let text = &source[node.byte_range()];
            // License headers (`/*\n* Copyright ... */`) keep their own style.
            let is_license = text.to_ascii_lowercase().contains("copyright");
            for (line_index, line) in text.split_inclusive('\n').enumerate() {
                if line_index > 0 && !is_license {
                    let whitespace = line
                        .bytes()
                        .take_while(|byte| matches!(byte, b' ' | b'\t'))
                        .count();
                    if line.as_bytes().get(whitespace) == Some(&b'*') && whitespace != expected {
                        violations.push(Violation {
                            file: String::new(),
                            line: node.start_position().row + line_index + 1,
                            col: whitespace + 2,
                            rule_id: self.id().into(),
                            message: "Initial star should align with start of block comment".into(),
                            auto_fixable: true,
                        });
                    }
                }
            }
        });
        violations
    }
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
        BlockCommentInitialStarAlignment.check(&tree, source)
    }

    #[test]
    fn aligned_comment_is_valid() {
        assert!(check("/*\n * aligned\n */\n").is_empty());
    }

    #[test]
    fn reports_each_misaligned_initial_star_at_star_end() {
        let violations = check("/*\n      * bad\n    */\n");
        assert_eq!(violations.len(), 2);
        assert_eq!((violations[0].line, violations[0].col), (2, 8));
        assert_eq!((violations[1].line, violations[1].col), (3, 6));
        assert!(violations.iter().all(|item| item.auto_fixable));
    }

    #[test]
    fn ignores_stars_that_are_not_initial() {
        assert!(check("/*\n   - inline * star\n */\n").is_empty());
    }
}
