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

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let spaces = line.len() - trimmed.len();

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
            let depth = expected_depth(&lines, i);
            let depth_expected = depth * is;
            // A closing brace sits one level inside its own block's depth —
            // except a mis-indented brace (non-multiple) which is reported at
            // the block's depth (covers continuation blocks like a `when {`
            // on a `=` continuation line).
            let expected_for_line = if trimmed.starts_with('}') && !too_shallow {
                depth_expected.saturating_sub(is)
            } else {
                depth_expected
            };
            if expected_for_line > spaces {
                let full_level_short = expected_for_line - spaces >= is;
                // Non-multiples report the brace-depth expectation even when
                // less than a full level short (ktlint reports a concrete
                // "should be N" for any wrong indent); multiple-of-N lines
                // only when a full level short (issue #152).
                if too_shallow || full_level_short {
                    too_shallow = true;
                    expected_indent = Some(expected_for_line);
                }
            }
            // Continuation lines (supertype after `:`, initializer after `=`,
            // expression body after `=`) are expected at the previous line's
            // indent + one level — the brace-depth expectation is one level
            // too shallow for them (issue #152).
            if too_shallow && expected_indent.is_none() {
                if let Some(prev_line) = lines.get(i.wrapping_sub(1)) {
                    let pt = prev_line.trim_end();
                    if (pt.ends_with(':') || pt.ends_with('=')) && !pt.ends_with(":=") {
                        let prev_indent = prev_line.len() - prev_line.trim_start().len();
                        let want = prev_indent + is;
                        if want > spaces {
                            expected_indent = Some(want);
                        }
                    }
                }
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
fn expected_depth(lines: &[&str], target: usize) -> usize {
    let mut depth = 0usize;
    // Block-comment and raw-string state span lines (a `/** ... */` KDoc often
    // contains `{`/`}` in code examples, and a `"""` raw string spans lines
    // with braces that must never count). Unterminated single-line strings
    // are invalid Kotlin and rejected earlier as parse errors.
    let mut in_block_comment = false;
    let mut in_raw_string = false;
    for (i, line) in lines.iter().enumerate() {
        if i >= target {
            break;
        }
        let t = line.trim();
        // Skip string content and comments: `{`/`}` inside quotes (or raw
        // strings), escaped quotes (`\"`), and comments are not block
        // delimiters and would inflate the depth.
        let mut in_string = false;
        let mut cleaned = String::with_capacity(t.len());
        let mut chars = t.chars().peekable();
        while let Some(c) = chars.next() {
            if !in_string && !in_block_comment && !in_raw_string {
                if c == '/' && chars.peek() == Some(&'/') {
                    // line comment: skip to end of line
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
                // escape: `\"` inside a string must not toggle in_string
                chars.next();
                continue;
            }
            if c == '\'' && !in_string && !in_raw_string {
                // character literal `'{'` — its interior is not code
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
                if !in_string && !in_raw_string
                    && chars.peek() == Some(&'"')
                    && chars.clone().nth(1) == Some('"')
                {
                    // `"""` opens a raw string (spans lines)
                    in_raw_string = true;
                    chars.next();
                    chars.next();
                    continue;
                }
                if in_raw_string {
                    if chars.peek() == Some(&'"') && chars.clone().nth(1) == Some('"') {
                        // `"""` closes the raw string
                        in_raw_string = false;
                        chars.next();
                        chars.next();
                        continue;
                    }
                    // a single quote inside a raw string is content
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
    }
    depth
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


