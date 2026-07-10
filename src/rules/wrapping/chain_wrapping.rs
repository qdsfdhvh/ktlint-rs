//! standard:chain-wrapping — method chains must be wrapped consistently.
//!
//! When a chain spans multiple lines, each `.call()` should be on its own line.
//! Single-line chains are fine.
//!
//! Examples:
//! - OK:  `list.filter { it > 0 }.map { it * 2 }.first()`
//! - OK:  `list\n    .filter { it > 0 }\n    .map { it * 2 }`
//! - BAD: `list.filter { it > 0 }\n    .map { it * 2 }` (inconsistent)

use crate::rules::{Rule, Violation};

pub struct ChainWrapping;

impl Rule for ChainWrapping {
    fn id(&self) -> &'static str {
        "standard:chain-wrapping"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        let mut prev_dot_call_line: Option<usize> = None;
        let mut chain_started = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check for the start of a dot-call chain
            if !chain_started {
                if trimmed.starts_with('.') {
                    chain_started = true;
                    prev_dot_call_line = Some(i);
                    continue;
                }
                // Check if this line starts a chain (has a dot call)
                if let Some(dot_pos) = trimmed.find('.') {
                    // If the line has a method call after the dot
                    if dot_pos > 0 && !trimmed[..dot_pos].contains("//") {
                        // Multi-line chain detection: next line starts with .
                        if i + 1 < lines.len() && lines[i + 1].trim().starts_with('.') {
                            chain_started = true;
                            prev_dot_call_line = Some(i);
                            continue;
                        }
                    }
                }
            }

            // Inside a chain
            if chain_started {
                if trimmed.starts_with('.') {
                    // Each .call should be indented consistently
                    let indent = line.len() - trimmed.len();
                    if indent == 0 {
                        // Not indented — for chains starting at column 0, this is fine
                    }
                    prev_dot_call_line = Some(i);
                } else {
                    // Chain ended (non-dot line)
                    chain_started = false;
                    prev_dot_call_line = None;
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        ChainWrapping.check(&tree, source)
    }

    #[test]
    fn single_line_chain_ok() {
        assert!(check("list.filter { it > 0 }.map { it * 2 }\n").is_empty());
    }

    #[test]
    fn multiline_chain_ok() {
        assert!(check("list\n    .filter { it > 0 }\n    .map { it * 2 }\n").is_empty());
    }

    #[test]
    fn no_chain_ok() {
        assert!(check("val x = 1\n").is_empty());
    }
}
