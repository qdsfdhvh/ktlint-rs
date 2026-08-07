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
#[allow(dead_code)]
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
        // tree-sitter-kotlin-sg predates Kotlin 2.2 context parameters. Mask
        // their unsupported prefix and normalize the declaration boundary only
        // for CST construction. Every replacement preserves byte offsets.
        let normalized = Self::normalize_context_parameters(source);
        self.parser
            .parse(normalized.as_deref().unwrap_or(source), None)
            .expect("Failed to parse Kotlin source")
    }

    #[allow(dead_code)]
    pub fn parse_file(&mut self, path: &Path) -> anyhow::Result<ParsedFile> {
        let source = std::fs::read_to_string(path)?;
        let tree = self.parse(&source);
        Ok(ParsedFile {
            path: Some(path.display().to_string()),
            source,
            tree,
        })
    }

    fn normalize_context_parameters(source: &str) -> Option<String> {
        let mut bytes = source.as_bytes().to_vec();
        let mut changed = false;
        let mut line_start = 0usize;
        for line in source.split_inclusive('\n') {
            let indent = line.len() - line.trim_start().len();
            let code = line.trim_start();
            if code.starts_with("context(") {
                if let Some(closing) = code.find(')') {
                    let prefix_start = line_start + indent;
                    let prefix_end = prefix_start + closing + 1;
                    bytes[prefix_start..prefix_end].fill(b' ');
                    changed = true;

                    let boundary = prefix_end;
                    let rest = &line[indent + closing + 1..];
                    if rest.starts_with(char::is_whitespace)
                        && !rest.starts_with('\n')
                        && !rest.trim_start().is_empty()
                    {
                        bytes[boundary] = b'\n';
                    }
                }
            }
            line_start += line.len();
        }
        changed.then(|| String::from_utf8(bytes).expect("source was valid UTF-8"))
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

    #[test]
    fn parse_kotlin_2_context_parameter() {
        let mut parser = KotlinParser::new();
        let tree = parser.parse("context(_: Foo) fun example() = Unit\n");
        assert!(!tree.root_node().has_error());
    }
}
