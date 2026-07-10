//! CST (Concrete Syntax Tree) utilities for walking tree-sitter Kotlin trees.
//!
//! Provides:
//! - Offset-to-line:col conversion
//! - Node type querying
//! - Whitespace inspection around nodes
//! - Common traversal patterns used by spacing rules

use tree_sitter::Node;

/// A position in source code: 1-based line and 1-based column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    pub line: usize,
    pub col: usize,
}

/// Context passed to every rule check, wrapping the source text and parsed tree.
pub struct CheckContext<'a> {
    pub source: &'a str,
    pub tree: &'a tree_sitter::Tree,
}

impl<'a> CheckContext<'a> {
    pub fn new(source: &'a str, tree: &'a tree_sitter::Tree) -> Self {
        Self { source, tree }
    }

    /// Convert a byte offset to a 1-based line:col position.
    pub fn offset_to_position(&self, offset: usize) -> SourcePosition {
        let mut line = 1;
        let mut col = 1;
        for (i, c) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        SourcePosition { line, col }
    }

    /// Get the text of a node.
    pub fn node_text(&self, node: &Node) -> &'a str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    /// Get the source text immediately before a node (up to `max_chars` chars).
    /// Returns (text, trailing_newline_count_before_text).
    pub fn text_before(&self, node: &Node, max_chars: usize) -> &'a str {
        let start = node.start_byte();
        let begin = start.saturating_sub(max_chars);
        &self.source[begin..start]
    }

    /// Get the source text immediately after a node (up to `max_chars` chars).
    pub fn text_after(&self, node: &Node, max_chars: usize) -> &'a str {
        let end = node.end_byte();
        let finish = (end + max_chars).min(self.source.len());
        &self.source[end..finish]
    }

    /// Check if the character immediately before `node` is a space.
    pub fn has_space_before(&self, node: &Node) -> bool {
        if node.start_byte() == 0 {
            return false;
        }
        self.source.as_bytes().get(node.start_byte() - 1) == Some(&b' ')
    }

    /// Check if the character immediately after `node` is a space.
    pub fn has_space_after(&self, node: &Node) -> bool {
        self.source.as_bytes().get(node.end_byte()) == Some(&b' ')
    }

    /// Check if the character immediately before `node` is a newline.
    pub fn starts_on_new_line(&self, node: &Node) -> bool {
        if node.start_byte() == 0 {
            return true;
        }
        self.source.as_bytes().get(node.start_byte() - 1) == Some(&b'\n')
    }

    /// Count consecutive spaces before `node`.
    pub fn leading_spaces(&self, node: &Node) -> usize {
        let mut count = 0;
        let mut pos = node.start_byte();
        while pos > 0 {
            pos -= 1;
            if self.source.as_bytes()[pos] == b' ' {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// Get the entire line containing `offset`.
    pub fn line_at(&self, offset: usize) -> &'a str {
        let line_start = self.source[..offset]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let line_end = self.source[offset..]
            .find('\n')
            .map(|i| offset + i)
            .unwrap_or(self.source.len());
        &self.source[line_start..line_end]
    }

    /// Walk all descendant nodes, calling `f` for each.
    pub fn walk_nodes<F>(&self, f: &mut F)
    where
        F: FnMut(Node<'a>),
    {
        let mut cursor = self.tree.root_node().walk();
        'outer: loop {
            let node = cursor.node();
            f(node);

            if cursor.goto_first_child() {
                continue;
            }
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    break 'outer;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_to_position() {
        let source = "line1\nline2\nline3\n";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_kotlin_sg::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let ctx = CheckContext::new(source, &tree);

        assert_eq!(
            ctx.offset_to_position(0),
            SourcePosition { line: 1, col: 1 }
        );
        assert_eq!(
            ctx.offset_to_position(6),
            SourcePosition { line: 2, col: 1 }
        );
    }

    #[test]
    fn test_has_space_before() {
        let source = "val x = 1";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_kotlin_sg::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let ctx = CheckContext::new(source, &tree);

        // Find the '=' node
        let mut found_eq = false;
        ctx.walk_nodes(&mut |node| {
            if node.kind() == "=" && !found_eq {
                found_eq = true;
                assert!(ctx.has_space_before(&node));
                assert!(ctx.has_space_after(&node));
            }
        });
        assert!(found_eq);
    }
}
