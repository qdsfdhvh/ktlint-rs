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
/// True when a trimmed line opens a multiline `for` header (`for (` or
/// `label@ for (`). ktlint indents the first continuation line of a for
/// header at the statement's own level, unlike every other paren list.
fn is_for_header(t: &str) -> bool {
    let t = t.trim();
    // Strip a leading label (`outer@ for (`).
    let t = if let Some(idx) = t.find('@') {
        let rest = t[idx + 1..].trim_start();
        if rest.starts_with("for ") || rest.starts_with("for(") {
            rest
        } else {
            t
        }
    } else {
        t
    };
    t.starts_with("for ") || t.starts_with("for(")
}

pub(crate) fn compute_line_expected(lines: &[&str], is: usize) -> Vec<usize> {
    let mut out = vec![0usize; lines.len()];
    let mut depth = 0usize;
    let mut prev_expected = 0usize;
    let mut paren_depth = 0usize;
    let mut paren_expected: Vec<(usize, bool, usize)> = Vec::new();
    let mut prev_last_code: Option<char> = None;
    let mut in_block_comment = false;
    let mut in_raw_string = false;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        // Strip strings, comments, and char literals first so the paren counts
        // below reflect code only — a `)` inside a string must not close a
        // paren list. Counts are accumulated inline to keep this hot path
        // single-pass (no post-hoc scan of a cleaned string).
        let mut in_string = false;
        let mut brace_opens = 0usize;
        let mut brace_closes = 0usize;
        // Same-line paren pairs cancel out; an unmatched `)` closes a
        // cross-line list (popping the stack right here), a trailing `(`
        // opens one (pushed at the end of the loop). last_code is the last
        // char that was counted (comments/strings leave it unset, so a KDoc
        // line ending in `(` never opens a list).
        let mut paren_local = 0usize;
        let mut last_code: Option<char> = None;
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
                        // A run of four quotes closes the raw string as a
                        // whole (`""""` = content ending in `"` + `"""`
                        // — ktor fixtures use this for a string containing a
                        // trailing quote). Consume the fourth quote too so no
                        // dangling `"` opens a regular string.
                        if chars.peek() == Some(&'"') {
                            chars.next();
                        }
                        continue;
                    }
                    continue;
                }
                in_string = !in_string;
                continue;
            }
            // String and raw-string content is data: nothing inside it may
            // count as a brace/paren or set last_code (a base64 padding `=`
            // or a `)` in a string must not open a continuation / close a
            // paren list).
            if in_string || in_raw_string {
                continue;
            }
            match c {
                '{' => brace_opens += 1,
                '}' => brace_closes += 1,
                '(' => paren_local += 1,
                ')' => {
                    // A line may close a paren list without *ending* in `)`
                    // — `) {`, `): Int {`, `),` all close it, and `) = foo(`
                    // closes one list and opens another (issue #176).
                    if paren_local > 0 {
                        paren_local -= 1;
                    } else {
                        paren_expected.pop();
                        paren_depth = paren_depth.saturating_sub(1);
                    }
                }
                _ => {}
            }
            last_code = Some(c);
        }
        let mut e = depth * is;
        if paren_depth > 0 {
            if let Some(&(list, for_header, opener_row)) = paren_expected.last() {
                // ktlint keeps a multiline `for (` header's structure at the
                // for-statement's own indent — the first line after the
                // opener (`for (\n    x in y`) and closing `)` lines of
                // nested lists inside the header — unlike function/if/while
                // lists which indent them. Continuation lines of the header
                // expression (`.dropWhile { … }` chains) get the list indent.
                let first_of_for_header = for_header && i == opener_row + 1;
                let closing_of_for_header = for_header && t.starts_with(')');
                if !(first_of_for_header || closing_of_for_header) && list > e {
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
                } else if prev_last_code == Some('{') && prev_was_supertype {
                    // Class body opened on a supertype continuation line.
                    e = prev_expected;
                } else if matches!(
                    prev_last_code,
                    Some('{') | Some('(') | Some(':') | Some('=')
                ) {
                    // Body/continuation line (block body, parameter list,
                    // supertype colon, initializer/expression body): the
                    // opener's expectation + one level. prev_last_code is the
                    // previous line's last *code* char — a trailing comment
                    // ending in `=`/`:` must not open a continuation.
                    let want = prev_expected.saturating_add(is);
                    if want > e {
                        e = want;
                    }
                }
            }
        }
        out[i] = e;
        prev_expected = e;
        prev_last_code = last_code;
        // Count this line's braces outside strings/comments.
        depth = depth
            .saturating_add(brace_opens)
            .saturating_sub(brace_closes);
        if last_code == Some('(') {
            paren_expected.push((e.saturating_add(is), is_for_header(t), i));
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

    // Issue #176: multiline value parameter lists must not push the closing
    // `)` line or the body that follows one level too far right. 0.1.12 was
    // clean; 0.1.13 regressed.
    #[test]
    fn multiline_value_parameter_list_clean() {
        let src = "fun f(\n    a: Int,\n) {\n    println(a)\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn multiline_value_parameter_list_no_trailing_comma() {
        let src = "fun f(\n    a: Int\n) {\n    println(a)\n}\n";
        assert!(check(src, 4).is_empty());
    }

    #[test]
    fn multiline_value_parameter_list_with_return_type() {
        let src = "fun f(\n    a: Int,\n): Int {\n    return a\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn multiline_class_parameters_clean() {
        let src = "class Wide(\n    private val alpha: String,\n    private val beta: String,\n) {\n    fun run(\n        a: Int,\n    ): Int {\n        return a\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn multiline_params_balanced_parens_in_default_value() {
        // A balanced `foo(1)` on a continuation line must not pop the outer
        // list; the `)` inside the default value is not a list closer.
        let src = "fun f(\n    a: Int = foo(1),\n    b: Int,\n) {\n    println(b)\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    // ── Regression battery: cases discovered while fixing issue #176 ──

    // `) = onEvent(` closes the parameter list AND opens a new call on the
    // same line. Net paren count is zero, but the close must pop the
    // parameter list and the trailing `(` must push the call list (okhttp
    // EventListenerAdapter shape).
    #[test]
    fn close_then_open_same_line() {
        let src = "class Adapter {\n    fun connectFailed(\n        call: Call,\n        ioe: IOException,\n    ) = onEvent(\n        ConnectFailed(\n            System.nanoTime(),\n            call,\n            ioe,\n        ),\n    )\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    // A KDoc line ending in `(` (a java example inside a block comment) must
    // not open a paren list — the push is driven by the last *code* char.
    #[test]
    fn kdoc_line_ending_in_paren_does_not_open_list() {
        let src = "/**\n * ```java\n *    public List<X509Certificate> checkServerTrusted(\n *        X509Certificate[] chain, String authType) {\n *    }\n * ```\n */\nclass Trust {\n    fun sslSocketFactory(\n        factory: SSLSocketFactory,\n    ) = apply {\n        if (factory != this.factory) {\n            this.factory = factory\n        }\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    // A trailing comment ending in `=`/`:` must not open a continuation.
    #[test]
    fn trailing_comment_ending_in_equals_does_not_continue() {
        let src = "class Hpack {\n    fun readIndexedHeaderField() {\n        bytesIn.writeByte(0xff) // == Indexed - Add ==\n        bytesIn.write(\"8080808008\".decodeHex()) // idx = -2147483521\n        assertFailsWith<IOException> {\n            hpackReader.readHeaders()\n        }.also { expected ->\n            assertThat(expected.message).isEqualTo(\"overflow\")\n        }\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    // ── Multiline `for (` headers (ktlint keeps the first line and the
    //    closing parens at the for-statement's own indent) ──

    #[test]
    fn for_header_first_line_at_statement_indent() {
        let src =
            "fun f() {\n    for (\n    step in seq()\n    ) {\n        println(step)\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn for_header_chained_continuation() {
        let src = "fun f() {\n    for (\n    step in generateSequence(1) { it * 2 }\n        .dropWhile { it < 64 }\n        .takeWhile { it <= 8192 }\n    ) {\n        bb.clear()\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn for_header_nested_list_close_at_statement_indent() {
        let src = "fun f() {\n    for (\n    x in listOf(\n        1,\n        2,\n    )\n    ) {\n        println(x)\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn for_header_labeled() {
        let src = "fun f() {\n    outer@ for (\n    step in seq()\n    ) {\n        println(step)\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn for_header_deep_nesting() {
        let src = "class Outer {\n    fun f() {\n        for (\n        step in seq()\n        ) {\n            println(step)\n        }\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn for_header_misindented_first_line_reported() {
        let src =
            "fun f() {\n    for (\n      step in seq()\n    ) {\n        println(step)\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.iter()
                .any(|x| x.line == 3 && x.message.contains("(6) (should be 4)")),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    // String content must not leak into paren/continuation state: a base64
    // padding `=` or a `)` inside a string must not open a continuation or
    // close a paren list.
    #[test]
    fn string_content_does_not_leak() {
        let src = "class T {\n    fun f() {\n        val bytes =\n          (\n            \"MIGJAoGBAICkUeG2stqfbyr6gyiVm5pN9YEDRXlowi+rfYGyWhC7ouW9fXAnhgShQKMOU8\" +\n              \"62mG3tcttSYGdsjM3z1crhQlUzpKqncrzwqbzPuAyt2t9Oib/bvjAvbl8gJH7IMRDl9RVgGYkApdkXVqgjSYigTH\" +\n              \"TEWxCEgnrfu/YzEkO6l3rXAgMBAAE=\"\n          ).decodeBase64()!!\n        println(bytes)\n    }\n}\n";
        let v = check(src, 2);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    // A raw string whose closing side is a four-quote run (`""""` =
    // content ending in `"` + closing `"""`) must not leave a dangling
    // `"` that opens a regular string and poisons the next lines.
    #[test]
    fn raw_string_four_quote_close() {
        let src = "fun f() {\n    assertEquals(\"\"\"\"1\"\"kotlin\"\"\"\", x)\n    assertEquals(\n        \"\"\"{\"id\":1}\"\"\",\n        y,\n    )\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn string_content_paren_does_not_close_list() {
        // `a: String = ")"` — the `)` inside the string is content.
        let src = "fun f(\n    a: String = \")\",\n    b: Int,\n) {\n    println(b)\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    // while/if multiline headers DO indent their first line (only `for` is
    // special in ktlint).
    #[test]
    fn while_and_if_headers_indent_first_line() {
        let src = "fun f(x: Int) {\n    while (\n        cond\n    ) {\n        println()\n    }\n    if (\n        x > 0\n    ) {\n        println(x)\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn secondary_constructor_multiline_clean() {
        let src = "class C(\n    val a: Int,\n) {\n    constructor(\n        a: Int,\n    ) : this()\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }
}
