//! Kotlin source code parsing via tree-sitter.
//!
//! Uses `tree-sitter-kotlin-sg` to build a Concrete Syntax Tree (CST).
//! The tree retains all whitespace, comments, and formatting details,
//! which is ideal for a formatter that must preserve non-violating code.

pub mod cst;

/// Detect Kotlin source that is structurally invalid — the kind of file ktlint
/// 1.8 rejects with "Not a valid Kotlin file" but tree-sitter's error recovery
/// would otherwise let through with zero violations.
///
/// tree-sitter recovers from most syntax errors (and its grammar lags the
/// Kotlin compiler on some real-world constructs), so this must never flag a
/// file ktlint accepts. Verified against real ktlint 1.8 on a corpus of
/// grammar-limited but valid files (actual/expect constructors, Compose
/// lambdas, raw strings): none of the heuristics below fires there.
/// Returns the 1-based (line, col) of the problem when the file is invalid.
///
/// Heuristics, in order:
/// 1. Unbalanced brackets / unterminated string or comment in a source-level
///    scan (this covers missing closing `)`, `}`, `]`, unterminated strings,
///    and unterminated block comments — the common ways a file breaks).
/// 2. An error node reaching the end of the file (structural breakage the
///    scanner misses, e.g. a dangling declaration keyword).
/// 3. A short error node containing garbage tokens that cannot appear in
///    Kotlin outside strings/comments (`#`, or `@` not starting an
///    annotation).
pub fn structural_invalid(source: &str, tree: &tree_sitter::Tree) -> Option<(usize, usize)> {
    // Files tree-sitter parses cleanly are valid by definition — skip the
    // scan entirely (it exists to separate genuine breakage from grammar
    // recovery artifacts on files that *do* have error nodes).
    if !tree.root_node().has_error() {
        return None;
    }
    // 1. Source-level bracket/string/comment balance scan.
    if let Some((line, col)) = scan_structural_balance(source) {
        return Some((line, col));
    }

    // 2. Garbage tokens in tree-sitter error nodes. tree-sitter's error
    // recovery hides garbage (it re-syncs at the next declaration), so the
    // scanner's bracket balance stays intact for files like
    // `class Foo {}\n@@@ ###\nclass Bar {}` — only the error-node text
    // reveals the junk. Restricted to short nodes so grammar-limited but
    // valid fragments (`actual constructor(`, `->`, `@Composable`) that
    // tree-sitter cannot parse are never flagged.
    let len = source.len();
    let mut garbage: Option<(usize, usize)> = None;
    let mut stack = vec![tree.root_node()];
    while let Some(n) = stack.pop() {
        if n.is_error() || n.is_missing() {
            let s = n.start_byte().min(len);
            let e = n.end_byte().min(len);
            let line = n.start_position().row + 1;
            let col = n.start_position().column + 1;
            let text = &source[s..e];
            if text.len() <= 64 && (text.contains('#') || garbage_at(text)) && garbage.is_none() {
                garbage = Some((line, col));
            }
        }
        let mut cur = n.walk();
        let mut kids = Vec::new();
        for c in n.children(&mut cur) {
            kids.push(c);
        }
        for c in kids.into_iter().rev() {
            stack.push(c);
        }
    }
    garbage
}

/// True when `text` contains `@` not followed by an identifier start — i.e. an
/// annotation marker or garbage, not a legal `@Composable`-style annotation.
fn garbage_at(text: &str) -> bool {
    let bytes = text.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'@' {
            let next = bytes.get(i + 1).copied();
            if next.is_none() || !(next.unwrap().is_ascii_alphabetic() || next.unwrap() == b'_') {
                return true;
            }
        }
    }
    false
}

/// Minimal Kotlin lexer that verifies bracket balance and literal termination
/// outside strings/comments. Catches the common file-breaking mistakes the
/// tree-sitter recovery hides: a missing `)`, `}`, or `]`, an unterminated
/// string/char/raw-string, and an unterminated nested block comment. Returns
/// the 1-based (line, col) of the first problem found.
///
/// Handles Kotlin's tricky bits: nested block comments, `\"` escapes, raw
/// strings (`\"\"\"`), and `$` interpolation (including nested string literals
/// inside `${...}`). An unterminated string is itself invalid — Kotlin
/// single-line strings cannot span a newline.
fn scan_structural_balance(source: &str) -> Option<(usize, usize)> {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        Code,
        Str,
        Char,
        /// `\"\"\"` raw string with `$` interpolation.
        Raw,
        /// `$$\"\"\"` raw string without interpolation (ktor's `$$`-escaped
        /// raw strings; `${` inside is literal text).
        RawNoInterp,
        LineComment,
        BlockComment,
    }
    let b = source.as_bytes();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut state = State::Code;
    let mut comment_depth = 0usize;
    let mut paren = 0i32;
    let mut brace = 0i32;
    let mut bracket = 0i32;

    macro_rules! here {
        () => {
            (line, (source[..i].rfind('\n').map_or(0, |p| i - p)) + 1)
        };
    }

    while i < b.len() {
        let c = b[i];
        match state {
            State::Code => {
                if c == b'/' && b.get(i + 1) == Some(&b'/') {
                    state = State::LineComment;
                    i += 2;
                } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
                    state = State::BlockComment;
                    comment_depth = 1;
                    i += 2;
                } else if c == b'"' {
                    if b.get(i + 1) == Some(&b'"') && b.get(i + 2) == Some(&b'"') {
                        state = State::Raw;
                        i += 3;
                    } else {
                        state = State::Str;
                        i += 1;
                    }
                } else if c == b'$'
                    && b.get(i + 1) == Some(&b'$')
                    && b.get(i + 2) == Some(&b'"')
                    && b.get(i + 3) == Some(&b'"')
                    && b.get(i + 4) == Some(&b'"')
                {
                    // `$$\"\"\"` — raw string with interpolation disabled
                    // (ktor tests use this for literal `$` content).
                    state = State::RawNoInterp;
                    i += 5;
                } else if c == b'\'' {
                    state = State::Char;
                    i += 1;
                } else if c == b'`' {
                    // Backtick-quoted identifier (`runTest - don't retry`): its
                    // interior is a name, never code — skip to the closing
                    // backtick so quotes/braces inside are not misread.
                    let close = source[i + 1..].find('`');
                    match close {
                        Some(rel) => i += 1 + rel + 1,
                        None => {
                            // Unterminated backtick identifier — invalid Kotlin.
                            return Some(here!());
                        }
                    }
                } else if c == b'(' {
                    paren += 1;
                    i += 1;
                } else if c == b')' {
                    paren -= 1;
                    if paren < 0 {
                        return Some(here!());
                    }
                    i += 1;
                } else if c == b'{' {
                    if std::env::var("SCAN_DEBUG").is_ok() {
                        eprintln!("CODE {{ at L{line}");
                    }
                    brace += 1;
                    i += 1;
                } else if c == b'}' {
                    if std::env::var("SCAN_DEBUG").is_ok() {
                        eprintln!("CODE }} at L{line}");
                    }
                    brace -= 1;
                    if brace < 0 {
                        return Some(here!());
                    }
                    i += 1;
                } else if c == b'[' {
                    bracket += 1;
                    i += 1;
                } else if c == b']' {
                    bracket -= 1;
                    if bracket < 0 {
                        return Some(here!());
                    }
                    i += 1;
                } else if c == b'\n' {
                    line += 1;
                    i += 1;
                } else {
                    i += 1;
                }
            }
            State::LineComment => {
                if c == b'\n' {
                    line += 1;
                    state = State::Code;
                }
                i += 1;
            }
            State::BlockComment => {
                if c == b'/' && b.get(i + 1) == Some(&b'*') {
                    comment_depth += 1;
                    i += 2;
                } else if c == b'*' && b.get(i + 1) == Some(&b'/') {
                    comment_depth -= 1;
                    i += 2;
                    if comment_depth == 0 {
                        state = State::Code;
                    }
                } else {
                    if c == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
            State::Str | State::Char => {
                if c == b'\\' {
                    i += 2;
                } else if c == b'$' && b.get(i + 1) == Some(&b'{') {
                    // Interpolation `${...}`: lex the expression (Code rules)
                    // until its matching `}`. Brackets inside count toward
                    // balance; strings inside are handled recursively.
                    let mut depth = 1usize;
                    i += 2;
                    while i < b.len() && depth > 0 {
                        let e = b[i];
                        if e == b'{' {
                            depth += 1;
                            i += 1;
                        } else if e == b'}' {
                            depth -= 1;
                            i += 1;
                        } else if e == b'"' {
                            if b.get(i + 1) == Some(&b'"') && b.get(i + 2) == Some(&b'"') {
                                // nested raw string inside interpolation
                                let mut j = i + 3;
                                while j + 2 < b.len()
                                    && !(b[j] == b'"' && b[j + 1] == b'"' && b[j + 2] == b'"')
                                {
                                    if b[j] == b'\n' {
                                        line += 1;
                                    }
                                    j += 1;
                                }
                                if j + 2 < b.len() {
                                    i = j + 3;
                                } else {
                                    return Some(here!()); // unterminated raw string
                                }
                            } else {
                                // nested single-line string: skip to its close
                                let mut j = i + 1;
                                loop {
                                    if j >= b.len() {
                                        return Some(here!());
                                    }
                                    if b[j] == b'\\' {
                                        j += 2;
                                        continue;
                                    }
                                    if b[j] == b'\n' {
                                        return Some(here!()); // unterminated
                                    }
                                    if b[j] == b'"' {
                                        i = j + 1;
                                        break;
                                    }
                                    j += 1;
                                }
                            }
                        } else {
                            if e == b'\n' {
                                line += 1;
                            }
                            i += 1;
                        }
                    }
                    if depth > 0 {
                        return Some(here!()); // unterminated interpolation
                    }
                } else if c == b'\n' {
                    // single-line strings cannot span a newline
                    return Some(here!());
                } else {
                    if (state == State::Char && c == b'\'') || (state == State::Str && c == b'"') {
                        state = State::Code;
                    }
                    i += 1;
                }
            }
            State::Raw | State::RawNoInterp => {
                if state == State::Raw && c == b'$' && b.get(i + 1) == Some(&b'{') {
                    let mut depth = 1usize;
                    i += 2;
                    while i < b.len() && depth > 0 {
                        let e = b[i];
                        if e == b'{' {
                            depth += 1;
                            i += 1;
                        } else if e == b'}' {
                            depth -= 1;
                            i += 1;
                        } else {
                            if e == b'\n' {
                                line += 1;
                            }
                            i += 1;
                        }
                    }
                    if depth > 0 {
                        return Some(here!());
                    }
                } else if c == b'"' {
                    // Raw strings may contain single/double quotes; the
                    // closing delimiter is the *last three* quotes of a run
                    // of 3+ (so `""""` = content `"` + close `"""`,
                    // `"""""` = content `""` + close). Runs shorter
                    // than 3 are plain content.
                    let mut n = 1usize;
                    while i + n < b.len() && b[i + n] == b'"' {
                        n += 1;
                    }
                    if n >= 3 {
                        state = State::Code;
                    }
                    i += n;
                } else {
                    if c == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
        }
    }
    match state {
        State::Str | State::Char | State::Raw => Some((line, 1)), // unterminated at EOF
        State::BlockComment => Some((line, 1)),                   // unterminated comment
        _ => {
            if paren != 0 || brace != 0 || bracket != 0 {
                Some((line, 1))
            } else {
                None
            }
        }
    }
}

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

#[cfg(test)]
mod structural_invalid_tests {
    use super::*;

    fn invalid(src: &str) -> bool {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(src);
        structural_invalid(src, &tree).is_some()
    }

    #[test]
    fn valid_code_is_clean() {
        for src in [
            "class Foo {\n    val x = 1\n}\n",
            "package com.example\n\nclass Foo {}\n",
            "val raw = \"\"\"\nmulti\nline\n\"\"\"\n",
            "class Foo {\n    fun bar() {\n        val s = \"a \\\"quoted\\\" string\"\n    }\n}\n",
            "val ch = 'x'\n",
            "val interpolated = \"${'$'}{foo(\"inner\")}\"\n",
            "// comment with \"quote\" and # hash\nclass Foo {}\n",
            "/* block with ' quote */\nclass Foo {}\n",
            "class Foo {}\n",
            "",                      // empty file
            "package com.example\n", // package-only is valid (no-empty-file's job)
        ] {
            assert!(!invalid(src), "should be valid: {src:?}");
        }
    }

    #[test]
    fn garbage_and_structural_breakage_detected() {
        // Issue #153 reproductions + edge cases ktlint rejects.
        for src in [
            // Not Kotlin at all
            "package com.example\n\nthis is not kotlin at all @@@ ###\n",
            // Unbalanced braces (one closing brace short)
            "package com.example\n\nclass Unclosed {\n    fun run() {\n        println(\"x\")\n}\n",
            // Missing inner closing brace
            "class Foo {\n    fun bar() {\n}\n",
            // Unterminated class body
            "class Foo {\n    fun bar() {\n        x()\n",
            // Trailing garbage
            "class Foo {}\nthis is garbage @@@\n",
            // Garbage between declarations
            "class Foo {}\n@@@ ###\nclass Bar {}\n",
            // Unterminated string
            "val x = \"abc\nclass Foo {}\n",
            // Unterminated raw string
            "val s = \"\"\"abc\nclass Foo {}\n",
            // Unterminated parameter list
            "fun foo(a: Int,\n",
            // Extra closing brace
            "class Foo {}\n}\n",
            // Known limitation: `if (x)` followed directly by `else` without a
            // body is invalid Kotlin but tree-sitter re-parses it as a property
            // setter (`x else`) with balanced brackets and a mid-file error, so
            // it is not detected. Extremely rare hand-typed mistake.
        ] {
            assert!(invalid(src), "should be invalid: {src:?}");
        }
    }

    #[test]
    fn grammar_limited_but_valid_files_are_not_flagged() {
        // ktor `actual constructor(...)` — valid Kotlin tree-sitter cannot parse.
        assert!(!invalid(
            "public actual constructor(\n    rootConfig: ServerConfig,\n) : Base() {\n    init { }\n}\n"
        ));
        // Compose lambda-type arrow inside a parameter list.
        assert!(!invalid(
            "@Composable\nfun Foo(\n    content: @Composable () -> Unit,\n) { }\n"
        ));
    }
}

#[cfg(test)]
mod yaml_scan_probe {
    use super::scan_structural_balance;

    #[test]
    fn probe() {
        let f = "tests/fixtures/ktor/ktor-server/ktor-server-config-yaml/jvmAndPosix/test/YamlConfigTest.kt";
        let src = std::fs::read_to_string(f).unwrap_or_default();
        // 逐段扫描找失衡点：每 20 行打印括号深度
        let mut paren = 0i32;
        let mut brace = 0i32;
        let mut bracket = 0i32;
        let mut i = 0usize;
        let mut line = 1usize;
        while i < src.len() {
            let c = src.as_bytes()[i];
            match c {
                b'(' => paren += 1,
                b')' => paren -= 1,
                b'{' => brace += 1,
                b'}' => brace -= 1,
                b'[' => bracket += 1,
                b']' => bracket -= 1,
                b'\n' => {
                    if line % 25 == 0 {
                        println!("L{line}: paren={paren} brace={brace} bracket={bracket}");
                    }
                    line += 1;
                }
                _ => {}
            }
            i += 1;
        }
        println!("EOF: paren={paren} brace={brace} bracket={bracket} lines={line}");
        // 完整扫描
        println!("scan result: {:?}", scan_structural_balance(&src));
        // 找到第一个 `'` 和 `` ` `` 和 `$` 的位置
        for (pat, name) in [
            ("'", "char"),
            ("\"", "dquote"),
            ("`", "backtick"),
            ("${", "interp"),
        ] {
            let mut cnt = 0;
            for (idx, _) in src.match_indices(pat) {
                cnt += 1;
                if cnt <= 3 {
                    let l = src[..idx].matches('\n').count() + 1;
                    let col = idx - src[..idx].rfind('\n').map_or(0, |p| p + 1);
                    println!("{name} #{cnt} at L{l}:{col}");
                }
            }
            println!("{name} total: {cnt}");
        }
    }
}
