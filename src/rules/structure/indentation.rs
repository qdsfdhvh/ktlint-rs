use crate::rules::{Rule, Violation};

/// JVM-compatible indentation check.
///
/// Core logic: for each line of code, check that the indentation (leading spaces)
/// is a multiple of the indent_size. Skip empty lines, comments, annotations,
/// KDoc, and KTS files.
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
        let mut in_block_comment = false;
        let mut in_raw_string = false;

        // Detect KTS files: if no class/fun/object declarations, skip indent.
        // Skip leading modifiers (public/private/sealed/data/...) before
        // checking the first keyword — `public sealed interface` must count.
        let is_kts = !lines.iter().any(|l| {
            let t = l.trim();
            if t.is_empty() || t.starts_with("//") || t.starts_with("/*") || t.starts_with("*") {
                return false;
            }
            let mut words = t.split_whitespace();
            let mut kw = words.next().unwrap_or("");
            while matches!(
                kw,
                "public"
                    | "private"
                    | "protected"
                    | "internal"
                    | "open"
                    | "abstract"
                    | "final"
                    | "sealed"
                    | "data"
                    | "inline"
                    | "external"
                    | "const"
                    | "suspend"
                    | "override"
                    | "companion"
                    | "annotation"
                    | "value"
                    | "expect"
                    | "actual"
            ) {
                kw = words.next().unwrap_or("");
            }
            matches!(
                kw,
                "class" | "fun" | "object" | "interface" | "enum" | "data"
            )
        });
        if is_kts {
            return violations;
        }

        // Per-line expected indent: brace depth × indent_size, raised to the
        // previous line's expectation + one level when the previous line ends
        // a continuation (`:`/`=`) or opens a block (`{`). This unifies
        // standard blocks and continuation blocks — a `when {` on a `=`
        // continuation line puts its body one level deeper than the raw brace
        // depth would suggest (issue #152).
        let line_expected = compute_line_expected(&lines, is);

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            // Trailing whitespace must not count as indentation (issue #169).
            let spaces = line.len() - line.trim_start().len();

            let raw_delimiters = line.matches("\"\"\"").count();
            if in_raw_string || raw_delimiters % 2 == 1 {
                if raw_delimiters % 2 == 1 {
                    in_raw_string = !in_raw_string;
                }
                continue;
            }

            // Track block comments
            if trimmed.starts_with("/*") {
                in_block_comment = true;
            }
            if in_block_comment {
                if trimmed.ends_with("*/") {
                    in_block_comment = false;
                }
                continue;
            }

            // Skip: blank, comments, annotations, KDoc markers, string-only lines
            if trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with('@')
                || trimmed == "*/"
                || trimmed.starts_with("* ")
                || trimmed.starts_with("*/")
                || trimmed.starts_with('"')
                || trimmed.contains("get(")
                || trimmed.contains("set(")
            {
                continue;
            }

            // Core JVM logic: indent must be a multiple of indent_size, and
            // must also match the nesting depth — a body one level shallower
            // than its block (e.g. 4 spaces where 8 are required) is wrong
            // even though 4 is a multiple. ktlint 1.8 reports both.
            let mut too_shallow = false;
            let mut expected_indent: Option<usize> = None;
            if spaces % is != 0 {
                too_shallow = true;
            }
            // Nesting-depth check. Closing braces and continuation lines
            // (lines whose expected depth is unchanged but whose indent is a
            // continuation alignment) are skipped conservatively: only a line
            // a full level short of its brace depth is reported.
            let depth_expected = line_expected[i];
            // A closing brace sits one level inside its own block's depth —
            // except a mis-indented brace (non-multiple) which is reported at
            // the block's depth (covers continuation blocks like a `when {`
            // on a `=` continuation line).
            let expected_for_line = if trimmed.starts_with('}') && !too_shallow {
                depth_expected.saturating_sub(is)
            } else {
                depth_expected
            };
            // ktlint reports whenever the actual indent differs from the
            // expected one — over-indentation too (its message is always a
            // concrete "should be M"). Non-multiples report even when less
            // than a full level short; multiple-of-N lines only when a full
            // level short (issue #152). Over-indented lines still report at
            // the expected indent (the fixer never lowers, so they stay).
            if expected_for_line > spaces {
                let full_level_short = expected_for_line - spaces >= is;
                if too_shallow || full_level_short {
                    too_shallow = true;
                    expected_indent = Some(expected_for_line);
                }
            } else if too_shallow {
                too_shallow = true;
                expected_indent = Some(expected_for_line);
            }
            if too_shallow {
                let message = match expected_indent {
                    Some(want) => {
                        format!("Unexpected indentation ({}) (should be {})", spaces, want)
                    }
                    None => format!(
                        "Unexpected indentation ({}) (should be multiple of {})",
                        spaces, is
                    ),
                };
                violations.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message,
                    auto_fixable: true,
                });
            }
        }
        violations
    }
}

/// Brace depth before a given line — used to detect zero-indent lines inside
/// a block without full continuation analysis.
/// Per-line expected indent. For each line: the brace depth × indent_size,
/// raised to the previous line's expectation + one level when the previous
/// line ends with `{`, `:`, or `=` (a block opener, supertype colon, or
/// initializer/expression-body `=`). Brace counting skips strings, comments,
/// and char literals, like [`expected_depth`].
pub(crate) fn compute_line_expected(lines: &[&str], is: usize) -> Vec<usize> {
    let mut out = vec![0usize; lines.len()];
    let mut depth = 0usize;
    let mut prev_expected = 0usize;
    let mut paren_depth = 0usize;
    let mut paren_expected: Vec<usize> = Vec::new();
    let mut in_block_comment = false;
    let mut in_raw_string = false;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        let mut e = depth * is;
        if t.ends_with(')') {
            paren_expected.pop();
            paren_depth = paren_depth.saturating_sub(1);
        }
        if paren_depth > 0 {
            if let Some(&list) = paren_expected.last() {
                if list > e {
                    e = list;
                }
            }
        }
        if i > 0 {
            let prev = lines[i - 1].trim_end();
            // A comment or blank line never opens a continuation — a comment
            // ending in `:` or `{` must not raise the next line's expectation.
            let prev_trim = prev.trim_start();
            let prev_inert = prev_trim.is_empty()
                || prev_trim.starts_with("//")
                || prev_trim.starts_with("/*")
                || prev_trim.starts_with('*');
            // True when the previous line was a supertype continuation (its
            // own previous line ended with `:`). The class body `{` that
            // follows opens the *class* body (indent = the continuation's
            // expectation), not a nested block (continuation + 4).
            let prev_was_supertype = i > 1
                && lines[i - 2].trim_end().ends_with(':')
                && !lines[i - 2].trim_end().ends_with(":=");
            if t.starts_with('}') {
                // A closing brace sits at the block's indent: the previous
                // line's expectation minus one level (handles `} else if {`
                // and continuation blocks like a `when {` on a `=` line).
                e = prev_expected.saturating_sub(is);
            } else if !prev_inert {
                if paren_depth > 0 {
                    // Inside a paren list the expectation already came from
                    // the list indent; keep it.
                } else if prev.ends_with('{') && prev_was_supertype {
                    // Class body opened on a supertype continuation line.
                    e = prev_expected;
                } else if prev.ends_with('{')
                    || prev.ends_with('(')
                    || prev.ends_with(':')
                    || prev.ends_with('=')
                {
                    // Body/continuation line (block body, parameter list,
                    // supertype colon, initializer/expression body): the
                    // opener's expectation + one level.
                    let want = prev_expected.saturating_add(is);
                    if want > e {
                        e = want;
                    }
                }
            }
        }
        out[i] = e;
        prev_expected = e;
        // Count this line's braces outside strings/comments.
        let mut in_string = false;
        let mut cleaned = String::with_capacity(t.len());
        let mut chars = t.chars().peekable();
        while let Some(c) = chars.next() {
            if !in_string && !in_block_comment && !in_raw_string {
                if c == '/' && chars.peek() == Some(&'/') {
                    break;
                }
                if c == '/' && chars.peek() == Some(&'*') {
                    in_block_comment = true;
                    chars.next();
                    continue;
                }
            }
            if in_block_comment {
                if c == '*' && chars.peek() == Some(&'/') {
                    in_block_comment = false;
                    chars.next();
                }
                continue;
            }
            if c == '\\' {
                chars.next();
                continue;
            }
            if c == '\'' && !in_string && !in_raw_string {
                loop {
                    match chars.next() {
                        Some('\\') => {
                            chars.next();
                        }
                        Some('\'') | None => break,
                        _ => {}
                    }
                }
                continue;
            }
            if c == '"' {
                if !in_string
                    && !in_raw_string
                    && chars.peek() == Some(&'"')
                    && chars.clone().nth(1) == Some('"')
                {
                    in_raw_string = true;
                    chars.next();
                    chars.next();
                    continue;
                }
                if in_raw_string {
                    if chars.peek() == Some(&'"') && chars.clone().nth(1) == Some('"') {
                        in_raw_string = false;
                        chars.next();
                        chars.next();
                        continue;
                    }
                    continue;
                }
                in_string = !in_string;
                continue;
            }
            if (in_string || in_raw_string) && (c == '{' || c == '}') {
                continue;
            }
            cleaned.push(c);
        }
        let opens = cleaned.bytes().filter(|b| *b == b'{').count();
        let closes = cleaned.bytes().filter(|b| *b == b'}').count();
        depth = depth.saturating_add(opens).saturating_sub(closes);
        if t.ends_with('(') {
            paren_expected.push(e.saturating_add(is));
            paren_depth += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(src: &str, indent_size: usize) -> Vec<Violation> {
        Indentation::new(indent_size).check(&KotlinParser::new().parse(src), src)
    }

    #[test]
    fn ok() {
        assert!(check("class Foo {\n    val x = 1\n}\n", 4).is_empty());
    }

    #[test]
    fn wrong_indent() {
        assert!(!check("fun a() {\n   val x = 1\n}\n", 4).is_empty());
    }

    #[test]
    fn kdoc_augmented() {
        let src = "/** doc\n * more\n */\nclass Foo\n";
        assert!(check(src, 4).is_empty());
    }

    #[test]
    fn block_comment_ignored() {
        assert!(check("/* comment\n   still comment */\nclass Foo\n", 4).is_empty());
    }

    #[test]
    fn annotation_ignored() {
        assert!(check("@Test\nfun foo() {}\n", 4).is_empty());
    }

    #[test]
    fn empty_lines_ignored() {
        assert!(check("\n\nclass Foo {}\n", 4).is_empty());
    }

    #[test]
    fn multiple_of_2() {
        let src = "class Foo {\n  val x = 1\n}\n";
        assert!(check(src, 2).is_empty());
    }

    #[test]
    fn wrong_multiple() {
        let src = "class Foo {\n   val x = 1\n}\n"; // 3 spaces, not multiple of 2
        assert!(!check(src, 2).is_empty());
    }

    #[test]
    fn else_if_combo() {
        let src =
            "fun f() {\n    if (x) {\n        a()\n    } else if (y) {\n        b()\n    }\n}\n";
        assert!(check(src, 4).is_empty());
    }

    #[test]
    fn lambda_continuation() {
        let src = "val x = list\n    .filter { it > 0 }\n    .map { it * 2 }\n";
        assert!(check(src, 4).is_empty());
    }

    #[test]
    fn tab_indent_detected() {
        let src = "class Foo {\n\tval x = 1\n}\n";
        assert!(!check(src, 4).is_empty()); // tab is not multiple of 4
    }

    #[test]
    fn continuation_indent() {
        let src = "fun f(x: Int,\n        y: String) {\n}\n";
        assert!(check(src, 4).is_empty());
    }

    // Ignored — KTS file with no class/fun declarations
    #[test]
    fn kts_ignored() {
        let src = "plugins {\n    id(\"com.android\")\n}\n";
        assert!(check(src, 4).is_empty());
    }

    #[test]
    fn modifier_prefixed_declarations_not_treated_as_kts() {
        // Issue #152: `public sealed interface` starts with a modifier, so the
        // old first-keyword KTS test skipped indentation entirely.
        let src =
            "public sealed interface Action {\n    public data object Open :\n      Action\n}\n";
        assert!(!check(src, 4).is_empty());
    }

    #[test]
    fn escaped_quote_in_string_does_not_inflate_depth() {
        // `\"{some_name\"` — the escaped quote must not end the string, so
        // `{` stays inside it and does not raise the expected depth.
        let src = "class A {\n    fun f() {\n        assertEquals(\n            \"a \\\"{x}\\\" b\",\n            y\n        )\n    }\n}\n";
        assert!(check(src, 4).is_empty());
    }

    #[test]
    fn char_literal_brace_does_not_inflate_depth() {
        let src = "class A {\n    fun f() {\n        val i = value.indexOf('{')\n        val p = value.substringBefore('{', \"\")\n        foo()\n    }\n}\n";
        let v = check(src, 4);
        if !v.is_empty() {
            println!(
                "VIOLATIONS: {:?}",
                v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
            );
        }
        assert!(v.is_empty());
    }

    #[test]
    fn apostrophe_inside_string_is_not_char_literal() {
        // `shouldn't` inside a string — the `'` must not start a char literal.
        let src = "class A {\n    fun f() {\n        require(length > 0) { \"range shouldn't be empty\" }\n        parts.add(x)\n    }\n}\n";
        assert!(check(src, 4).is_empty());
    }
}

