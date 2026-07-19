use crate::rules::{Rule, Violation};

/// Context-sensitive indentation check (JVM-compatible).
/// Tracks a stack of expected indent levels per brace block.
/// Reduces over-flagging by detecting continuation lines and expression lambdas.
pub struct Indentation {
    indent_size: usize,
}

impl Indentation {
    pub fn new(indent_size: usize) -> Self {
        Self { indent_size }
    }
}

impl Rule for Indentation {
    fn id(&self) -> &'static str {
        "standard:indent"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let is = self.indent_size;
        let lines: Vec<&str> = source.lines().collect();
        let mut stack: Vec<usize> = Vec::new();
        let mut in_block_comment = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let spaces = line.len() - trimmed.len();

            // Track block comments across lines
            if trimmed.starts_with("/*") {
                in_block_comment = true;
            }
            if in_block_comment {
                if trimmed.ends_with("*/") {
                    in_block_comment = false;
                }
                continue;
            }

            // Skip: blank, comments, annotations, KDoc continuation
            if trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with('@')
                || trimmed == "*/"
                || trimmed.starts_with("* ")
                || trimmed.starts_with("*/")
            {
                continue;
            }

            // --- Continuation detection ---
            // Lines that continue a previous expression don't need block-indent checking.
            if is_continuation_line(trimmed, i, &lines) {
                // Still track braces for continuation lines
                track_braces_on_line(trimmed, spaces, is, &mut stack);
                continue;
            }

            // --- Combo closing + opening brace (e.g. "} else {", "} catch(") ---
            let has_closing = trimmed.starts_with("}");
            let is_pure_close = trimmed == "}";
            if has_closing {
                let parent = stack.last().copied().unwrap_or(0).saturating_sub(is);
                if spaces != parent {
                    violations.push(violation(self.id(), i + 1, spaces, parent));
                }
                stack.pop();
                if is_pure_close {
                    continue;
                }
                // Fall through: process opening-brace part (e.g. " else {")
            }

            // --- Opening brace ---
            // Only track as a new block if this is a block-starting line,
            // not a mid-expression lambda or inline block.
            let opens_block = trimmed.ends_with('{')
                && !trimmed.ends_with("${")      // skip string templates
                && !trimmed.starts_with("//")    // skip comments
                && !is_inline_brace(trimmed); // skip inline lambdas/blocks

            // Re-evaluate expected indent AFTER combo-brace stack adjustment
            let expected = stack.last().copied().unwrap_or(0);
            // --- Indent check for non-brace, non-closing lines ---
            if !stack.is_empty() && spaces < expected {
                violations.push(violation(self.id(), i + 1, spaces, expected));
            }

            // After processing this line, if it opens a block, push expected indent
            if opens_block && is_kotlin_block_starter(trimmed) {
                let next_expected = spaces + is;
                stack.push(next_expected);
            }

            // Track braces on lines that have them but aren't block-openers
            // (e.g., inline lambdas: val x = list.map { it * 2 })
            if trimmed.ends_with('{') && !opens_block {
                track_braces_on_line(trimmed, spaces, is, &mut stack);
            }
        }

        violations
    }
}

/// Check if a line is a continuation of a previous expression.
fn is_continuation_line(trimmed: &str, i: usize, lines: &[&str]) -> bool {
    // Lines starting with these are always continuations
    if trimmed.starts_with('.')
        || trimmed.starts_with("?:")
        || trimmed.starts_with("&&")
        || trimmed.starts_with("||")
        || trimmed.starts_with("?:")
        || trimmed.starts_with(")")
        || trimmed.starts_with("]")
    {
        return true;
    }

    // Check if the line starts with a comma-separated parameter continuation
    // e.g.: "    b: Int," after "fun foo(a: Int,"
    if trimmed.starts_with(',') {
        return true;
    }

    // Check previous non-blank line for continuation markers
    if i > 0 {
        let mut prev_idx = i.saturating_sub(1);
        loop {
            if prev_idx == 0 {
                break;
            }
            let prev = lines[prev_idx].trim();
            if prev.is_empty() || prev.starts_with("//") || prev.starts_with("/*") {
                if prev_idx == 0 {
                    break;
                }
                prev_idx -= 1;
                continue;
            }
            // Previous line ends with continuation marker
            if prev.ends_with('(')
                || prev.ends_with(',')
                || prev.ends_with('=')
                || prev.ends_with("?.")
                || prev.ends_with("?:")
                || prev.ends_with("&&")
                || prev.ends_with("||")
                || prev.ends_with("->")
                || prev.ends_with('.')
                || prev.ends_with("+")
                || prev.ends_with("-")
                || prev.ends_with("*")
                || prev.ends_with("/")
            {
                return true;
            }
            // Previous line is a single `)` (closing paren of a wrapped call)
            if prev == ")" {
                return true;
            }
            break;
        }
    }

    false
}

/// Check if a `{` on this line is an inline/expression brace, not a block opener.
/// Inline braces include: lambdas as arguments, property delegates, when entries.
fn is_inline_brace(trimmed: &str) -> bool {
    // If there's meaningful content before `{`, it's likely inline
    // e.g.: "val x = foo {", "return bar {", "list.map {", "when (x) {"
    // Block openers: "fun foo() {", "class Foo {", "if (x) {", "} else {"

    // These are always block openers
    if trimmed.starts_with("if (")
        || trimmed.starts_with("else if (")
        || trimmed.starts_with("for (")
        || trimmed.starts_with("while (")
        || trimmed.starts_with("try {")
        || trimmed.starts_with("do {")
    {
        return false;
    }

    // Combo braces (after closing brace) are block openers
    if trimmed.starts_with("} ")
        || trimmed.starts_with("} else")
        || trimmed.starts_with("} catch")
        || trimmed.starts_with("} finally")
    {
        return false;
    }

    // If the line is just `{`, it's a block opener (though unusual)
    if trimmed == "{" {
        return false;
    }

    // If the `{` is the only brace and there's content like `=` before it,
    // it's inline (lambda, delegate, etc.)
    if trimmed.contains('=') {
        return true;
    }

    // `when` blocks: `when (x) {` — the `{` is part of the when expression
    if trimmed.starts_with("when ") || trimmed.starts_with("when(") {
        return false;
    }

    // Check: does the line look like a declaration header?
    // Declaration headers are block openers
    if trimmed.starts_with("class ")
        || trimmed.starts_with("interface ")
        || trimmed.starts_with("object ")
        || trimmed.starts_with("fun ")
        || trimmed.starts_with("init ")
        || trimmed.starts_with("companion object ")
        || trimmed.starts_with("enum class ")
        || trimmed.starts_with("data class ")
        || trimmed.starts_with("sealed class ")
        || trimmed.starts_with("sealed interface ")
        || trimmed.starts_with("abstract class ")
        || trimmed.starts_with("open class ")
        || trimmed.starts_with("annotation class ")
        || trimmed.starts_with("inline class ")
        || trimmed.starts_with("value class ")
        || trimmed.starts_with("expect class ")
        || trimmed.starts_with("actual class ")
    {
        return false;
    }

    // For anything else with a `{` that's not clearly a block opener → inline
    if trimmed.ends_with('{') {
        return true;
    }

    false
}

/// Track brace opening/closing for lines that have braces but aren't block-openers.
fn track_braces_on_line(trimmed: &str, spaces: usize, is: usize, stack: &mut Vec<usize>) {
    let mut depth: i32 = 0;
    let mut bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                depth += 1;
            }
            b'}' => {
                depth -= 1;
            }
            _ => {}
        }
        i += 1;
    }
    // If net more open than close, push a synthetic indent level
    if depth > 0 {
        stack.push(spaces + is);
    }
    // If net more close than open, pop
    for _ in 0..(-depth).max(0) {
        stack.pop();
    }
}

fn violation(rule_id: &str, line: usize, actual: usize, expected: usize) -> Violation {
    Violation {
        file: String::new(),
        line,
        col: 1,
        rule_id: rule_id.into(),
        message: format!(
            "Unexpected indentation ({}) (should be {})",
            actual, expected
        ),
        auto_fixable: true,
    }
}

/// Whether a trimmed line starts a real Kotlin block (not a DSL call).
/// Used to distinguish Kotlin code from Gradle KTS DSL.
fn is_kotlin_block_starter(trimmed: &str) -> bool {
    let kw = trimmed.split_whitespace().next().unwrap_or("");
    matches!(kw, "class" | "fun" | "if" | "for" | "while" | "when" | "try"
        | "do" | "object" | "interface" | "enum" | "data" | "sealed"
        | "abstract" | "open" | "companion" | "init" | "expect" | "actual")
        || trimmed.starts_with("else")
        || trimmed.starts_with("catch")
        || trimmed.starts_with("}")
        || trimmed.starts_with("finally")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str, indent_size: usize) -> Vec<Violation> {
        Indentation::new(indent_size).check(&crate::parser::KotlinParser::new().parse(src), src)
    }

    #[test]
    fn correct_indent() {
        assert!(check("fun a() {\n    val x = 1\n}\n", 4).is_empty());
    }

    #[test]
    fn wrong_indent() {
        assert!(!check("fun a() {\n  val x = 1\n}\n", 4).is_empty());
    }

    #[test]
    fn nested_blocks() {
        assert!(check(
            "class Foo {\n    fun bar() {\n        val x = 1\n    }\n}\n",
            4
        )
        .is_empty());
    }

    #[test]
    fn closing_brace_at_parent_level() {
        assert!(check("fun f() {\n    if (x) {\n        y()\n    }\n}\n", 4).is_empty());
    }

    #[test]
    fn continuation_allowed() {
        // Parameter continuation
        assert!(check("fun f(\n        a: Int,\n        b: Int\n) {}\n", 4).is_empty());
    }

    #[test]
    fn chained_call_continuation() {
        let src = "fun foo() {\n    list\n        .map { it * 2 }\n        .filter { it > 0 }\n}\n";
        assert!(check(src, 4).is_empty());
    }

    #[test]
    #[ignore]
    fn inline_lambda_not_block() {
        // Inline lambda as argument should not create indent expectations
        let src = "fun foo() {\n    val x = list.map { it\n        it * 2\n    }\n}\n";
        assert!(check(src, 4).is_empty());
    }

    #[test]
    fn else_if_combo() {
        assert!(check(
            "fun f() {\n    if (x) {\n        a()\n    } else if (y) {\n        b()\n    }\n}\n",
            4
        )
        .is_empty());
    }
}
