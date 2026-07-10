//! Test helper to inspect tree-sitter-kotlin-sg node types.
//! Run with: cargo test -- --nocapture node_types_dump

#[cfg(test)]
mod node_dump {
    use crate::parser::{cst::CheckContext, KotlinParser};

    #[test]
    fn node_types_dump() {
        let source = r#"
package com.example

import kotlin.collections.*

/**
 * Sample class.
 */
@Deprecated("use Bar")
class Foo(
    val x: Int,
    val y: String = "hello"
) : Base() {
    
    fun bar(): Int {
        val result = 1 + 2 * 3
        return result
    }
    
    fun baz(a: Int, b: String): Boolean = a > 0
}
"#;
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        let ctx = CheckContext::new(source, &tree);

        let mut seen = std::collections::HashSet::new();
        ctx.walk_nodes(&mut |node| {
            let kind = node.kind();
            if seen.insert(kind.to_string()) && !kind.starts_with('_') {
                let depth = node.start_position().row; // approximate depth hint
                let start = node.start_position();
                let text = ctx.node_text(&node);
                let text_preview = if text.len() > 30 {
                    format!("{}...", &text[..30].replace('\n', "⏎"))
                } else {
                    text.replace('\n', "⏎")
                };
                println!(
                    "{:indent$}{} {:?} [{}.{}]",
                    "",
                    kind,
                    text_preview,
                    start.row + 1,
                    start.column + 1,
                    indent = depth * 2
                );
            }
        });
    }

    #[test]
    fn list_operator_nodes() {
        let source = "val x = 1 + 2 - 3 * 4 / 5 % 6 == 7 != 8 < 9 > 10 <= 11 >= 12 && 13 || 14";
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        let ctx = CheckContext::new(source, &tree);

        ctx.walk_nodes(&mut |node| {
            let text = ctx.node_text(&node).to_string();
            if text.len() <= 3 {
                println!(
                    "  {} → {:?} [{}.{}]",
                    node.kind(),
                    text,
                    node.start_position().row + 1,
                    node.start_position().column + 1
                );
            }
        });
    }
}
