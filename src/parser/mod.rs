//! Kotlin source code parsing via tree-sitter.
//!
//! Uses `tree-sitter-kotlin-sg` to build a Concrete Syntax Tree (CST).
//! The tree retains all whitespace, comments, and formatting details,
//! which is ideal for a formatter that must preserve non-violating code.

pub mod cst;

use std::path::Path;

#[cfg(test)]
mod node_types_test;

/// A parsed Kotlin file.
pub struct ParsedFile {
    pub path: Option<String>,
    pub source: String,
    pub tree: tree_sitter::Tree,
}

/// Parser backed by tree-sitter-kotlin-sg.
pub struct KotlinParser {
    parser: tree_sitter::Parser,
}

impl KotlinParser {
    pub fn new() -> Self {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_kotlin_sg::LANGUAGE.into())
            .expect("Failed to load Kotlin tree-sitter grammar");
        Self { parser }
    }

    pub fn parse(&mut self, source: &str) -> tree_sitter::Tree {
        self.parser
            .parse(source, None)
            .expect("Failed to parse Kotlin source")
    }

    pub fn parse_file(&mut self, path: &Path) -> anyhow::Result<ParsedFile> {
        let source = std::fs::read_to_string(path)?;
        let tree = self.parse(&source);
        Ok(ParsedFile {
            path: Some(path.display().to_string()),
            source,
            tree,
        })
    }
}

impl Default for KotlinParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_class() {
        let mut parser = KotlinParser::new();
        let source = "class Foo(val x: Int)\n";
        let tree = parser.parse(source);
        let root = tree.root_node();
        assert_eq!(root.kind(), "source_file");
        assert!(root.child_count() > 0);
    }
}
