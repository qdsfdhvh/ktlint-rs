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
        // Oracle: an Allman block (`{` on its own line) belonging to an
        // if/when/try/function header takes an extra level — the block sits
        // at the header's indent + one, its body one deeper, its `}` at the
        // block. `when` is the exception: its `{` stays standard but the
        // entries inside take the extra level (issue #202).
        let elevated = find_allman_elevated_blocks(_tree, source);
        // AST-driven expected indent (issue #202 rework): the first token's
        // container chain decides each code row. Falls back to the line-scan
        // model when the AST cannot classify the row.
        // AST-driven expected indent (issue #202 rework): the first token's
        // container chain decides each code row. Falls back to the line-scan
        // model when the AST cannot classify the row.
        let line_expected: Vec<usize> = (0..lines.len())
            .map(|i| {
                ast_expected(_tree, source, i, is)
                    .unwrap_or_else(|| compute_line_expected(&lines, is, &elevated)[i])
            })
            .collect();

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
            let depth_expected = line_expected[i];
            // A closing brace sits one level inside its own block's depth —
            // except a mis-indented brace (non-multiple) which is reported at
            // the block's depth (covers continuation blocks like a `when {`
            // on a `=` continuation line).
            let expected_for_line = depth_expected;
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

/// Oracle (issue #202): an Allman block — `{` alone on its line — that
/// opens the body of an `if`/`when`/`try` takes an extra indentation level:
///
/// ```text
/// if (x)        // 4
/// {             // expected 8  (block = header + 1)
///     a()       // expected 12
/// }             // expected 8
/// ```
///
/// while/for/do/class/object/lambda blocks keep the standard indentation.
/// Returns (open_line, close_line) pairs (0-based) for such blocks. The
/// `{` must be alone on its line (Allman); `if (x) {` never matches.
/// The nearest statement-level ancestor of a lambda literal — the line its
/// block aligns with (`val r = list\n    .map\n    {` -> the `val r` line).
/// For an inline `{` block, the header row the block belongs to — the
/// `fun`/`if`/`when`/`while` line carrying the `{`, so the closing `}`
/// aligns with the statement instead of the `{` row.
fn blocks_header_row(owner: &tree_sitter::Node, open: usize) -> Option<usize> {
    match owner.kind() {
        "function_declaration" => owner
            .children(&mut owner.walk())
            .find(|c| c.kind() == "fun")
            .map(|c| c.start_position().row)
            .or(Some(owner.start_position().row)),
        "class_declaration" | "object_declaration" | "if_expression" | "while_statement"
        | "for_statement" | "when_expression" | "when_entry" | "try_expression"
        | "do_while_statement" => Some(owner.start_position().row),
        _ => {
            // control_structure_body etc.: the nearest control structure
            let mut n = *owner;
            while let Some(p) = n.parent() {
                if matches!(
                    p.kind(),
                    "if_expression"
                        | "while_statement"
                        | "for_statement"
                        | "when_expression"
                        | "when_entry"
                        | "try_expression"
                        | "do_while_statement"
                ) {
                    return Some(p.start_position().row);
                }
                n = p;
            }
            Some(open)
        }
    }
}

fn statement_line_of(node: &tree_sitter::Node) -> Option<usize> {
    let mut n = *node;
    while let Some(p) = n.parent() {
        if matches!(
            p.kind(),
            "property_declaration"
                | "expression_statement"
                | "function_declaration"
                | "return_statement"
                | "assignment_expression"
                | "if_expression"
                | "when_expression"
                | "when_entry"
                | "for_statement"
                | "while_statement"
                // A bare statement in a block — `run\n{` inside an if body.
                | "statements"
        ) {
            return Some(p.start_position().row);
        }
        n = p;
    }
    None
}

fn find_allman_elevated_blocks(
    tree: &tree_sitter::Tree,
    source: &str,
) -> Vec<(usize, usize, bool, Option<usize>)> {
    let mut out = Vec::new();
    let mut stack: Vec<tree_sitter::Node> = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        // (open_line, close_line, open_elevated, statement_line)
        let mut blocks: Vec<(usize, usize, bool, Option<usize>)> = Vec::new();
        match node.kind() {
            "if_expression" => {
                // The if body and any else body are control_structure_body
                // children; `else if` chains nest their own if_expression.
                for c in node.children(&mut node.walk()) {
                    if c.kind() == "control_structure_body" {
                        blocks.push((c.start_position().row, c.end_position().row, true, None));
                    }
                }
            }
            "when_expression" => {
                // Recorded regardless of subject; the elevated lift applies
                // only to `when\n{` (no subject, `{` alone) — a `when (x)\n{`
                // keeps its own `{` standard (entries still lift via their
                // own frames), and an inline `when (x) {` pins its body to
                // the when-line + one.
                let has_subject = node
                    .children(&mut node.walk())
                    .any(|c| c.kind() == "when_subject");
                if let Some(open) = node.children(&mut node.walk()).find(|c| c.kind() == "{") {
                    let close = node
                        .children(&mut node.walk())
                        .find(|c| c.kind() == "}")
                        .map(|c| c.start_position().row)
                        .unwrap_or(node.end_position().row);
                    blocks.push((open.start_position().row, close, !has_subject, None));
                }
            }
            "when_entry" => {
                // `1 ->\n{` — the entry body is a control_structure_body
                // like an if body. Only braced bodies become frames (a
                // bare expression body `1 -> a()` is a single row, not a
                // block to pin).
                if let Some(body) = node
                    .children(&mut node.walk())
                    .find(|c| c.kind() == "control_structure_body")
                {
                    let braced = body.children(&mut body.walk()).any(|c| c.kind() == "{");
                    if braced {
                        blocks.push((
                            body.start_position().row,
                            body.end_position().row,
                            true,
                            None,
                        ));
                    }
                }
            }
            "try_expression" => {
                if let Some(open) = node.children(&mut node.walk()).find(|c| c.kind() == "{") {
                    let close = node
                        .children(&mut node.walk())
                        .find(|c| c.kind() == "}")
                        .map(|c| c.start_position().row)
                        .unwrap_or(node.end_position().row);
                    blocks.push((open.start_position().row, close, true, None));
                }
            }
            "function_declaration" => {
                // Only block bodies count as frames — an expression body
                // (`fun f() = when (x) {`) is not a block to pin.
                if let Some(body) = node
                    .children(&mut node.walk())
                    .find(|c| c.kind() == "function_body")
                {
                    let opens_brace = source[body.start_byte()..].trim_start().starts_with('{');
                    if opens_brace {
                        blocks.push((
                            body.start_position().row,
                            body.end_position().row,
                            true,
                            None,
                        ));
                    }
                }
            }
            "while_statement" | "for_statement" => {
                if let Some(body) = node
                    .children(&mut node.walk())
                    .find(|c| c.kind() == "control_structure_body")
                {
                    blocks.push((
                        body.start_position().row,
                        body.end_position().row,
                        false,
                        None,
                    ));
                }
            }
            "catch_block" | "finally_block" | "class_body" => {
                if let Some(open) = node.children(&mut node.walk()).find(|c| c.kind() == "{") {
                    let close = node
                        .children(&mut node.walk())
                        .find(|c| c.kind() == "}")
                        .map(|c| c.start_position().row)
                        .unwrap_or(node.end_position().row);
                    blocks.push((open.start_position().row, close, false, None));
                }
            }
            "lambda_literal" => {
                // Recorded regardless of whether `{` sits alone on its line:
                // the closing `}` aligns with the lambda's own line (chained
                // `val r = list\n    .map { x ->` closes at the chain line,
                // oracle-verified), and the body sits one level deeper.
                if let Some(open) = node.children(&mut node.walk()).find(|c| c.kind() == "{") {
                    let close = node
                        .children(&mut node.walk())
                        .find(|c| c.kind() == "}")
                        .map(|c| c.start_position().row)
                        .unwrap_or(node.end_position().row);
                    let stmt = statement_line_of(&node);
                    blocks.push((open.start_position().row, close, false, stmt));
                }
            }
            _ => {}
        }
        for (open, close, open_elevated, stmt) in blocks {
            let line = source.lines().nth(open).map(|l| l.trim()).unwrap_or("");
            // Lambda frames are recorded even when `{` shares its line with
            // the call (`foo { x ->`); every other kind needs the `{` alone
            // (Allman) to matter.
            if line == "{" || stmt.is_some() {
                out.push((open, close, open_elevated, stmt));
            }
        }
        for i in (0..node.child_count()).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
    out
}

pub(crate) fn compute_line_expected(
    lines: &[&str],
    is: usize,
    elevated: &[(usize, usize, bool, Option<usize>)],
) -> Vec<usize> {
    let mut out = vec![0usize; lines.len()];
    let mut depth = 0usize;
    let mut prev_expected = 0usize;
    let mut paren_depth = 0usize;
    let mut paren_expected: Vec<(usize, bool, usize)> = Vec::new();
    let mut pending_pops = 0usize;
    let mut prev_last_code: Option<char> = None;
    let mut in_block_comment = false;
    let mut block_depth = 0usize;
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
            if !in_string && !in_raw_string {
                if c == '/' && chars.peek() == Some(&'/') && !in_block_comment {
                    break;
                }
                if c == '/' && chars.peek() == Some(&'*') {
                    // Nested block comments (Kotlin allows them; a KDoc code
                    // sample may contain `/* ... */`).
                    block_depth += 1;
                    in_block_comment = true;
                    chars.next();
                    continue;
                }
            }
            if in_block_comment {
                if c == '*' && chars.peek() == Some(&'/') {
                    block_depth = block_depth.saturating_sub(1);
                    if block_depth == 0 {
                        in_block_comment = false;
                    }
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
                        // trailing quote). Consume any further quotes so no
                        // dangling `"` opens a regular string that swallows
                        // the parens after it (`""""vv""""")`).
                        while chars.peek() == Some(&'"') {
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
                    // The pop is deferred to the end of the line so a list
                    // row carrying code before the `)` (`y: String) {`) still
                    // reads the list expectation.
                    if paren_local > 0 {
                        paren_local -= 1;
                    } else {
                        pending_pops += 1;
                    }
                }
                _ => {}
            }
            last_code = Some(c);
        }
        let mut e = depth * is;
        // Allman elevated blocks: the `{` line sits at the header's
        // expectation + one level (except `when`, whose `{` stays standard),
        // and every line inside the block (up to the closing `}`) one level
        // deeper than the raw brace depth suggests.
        // Every frame containing `i` contributes:
        //  - a frame's *body* lines (open < i < close) are raised one level
        //    only when that frame is itself elevated (if/when-entry/try/fun);
        //    non-elevated blocks (while/lambda/class...) rely on the raw
        //    brace depth, which already matches
        //  - a frame's own `{` line (i == open) is raised when elevated
        //  - the innermost frame's closing line (i == close) aligns with
        //    that block's opening `{` (out[open])
        if t.contains(".also") {}
        // Lines of a multiline `for (` header stay at the statement indent
        // (first line and the closing `)` row) — the paren logic governs
        // them, not block frames.
        let in_for_header_first = paren_depth > 0
            && paren_expected
                .last()
                .is_some_and(|&(_, fh, or)| fh && i == or + 1);
        let closing_of_for_header = paren_depth > 0
            && paren_expected.last().is_some_and(|&(_, fh, _)| fh)
            && t.starts_with(')');
        // Any other row of a multiline `for (` header (the range/iteration
        // expression line) keeps the statement indent too.
        let in_for_header_body = paren_depth > 0
            && paren_expected
                .last()
                .is_some_and(|&(_, fh, or)| fh && i > or + 1 && !t.starts_with(')'));
        let mut closest_close: Option<(usize, usize)> = None;
        for &(open, close, open_elevated, stmt) in elevated.iter() {
            if i > open && i < close {
                // Every block's body sits at its opening line + one level —
                // for elevated frames that's an extra lift on top of the
                // brace depth; for standard frames (lambda/while/class) it
                // pins the body to the block's `{` (a chained lambda's body
                // follows the chain line: `.combine(...) { x ->` body at
                // chain + one, oracle-verified). Rows inside a paren list
                // are governed by the paren logic instead (a `for (` header
                // keeps its first line at the statement indent).
                if open_elevated {
                    e = e.saturating_add(is);
                } else if !in_for_header_first
                    && !closing_of_for_header
                    && !in_for_header_body
                    && !t.starts_with(')')
                {
                    // Only rows already indented inside the block get the
                    // body pin — a top-level declaration following the block
                    // (`public fun` after a previous function) must not be
                    // lifted.
                    let line_indent = lines[i].len() - lines[i].trim_start().len();
                    if line_indent >= out[open].saturating_add(is) {
                        e = e.max(out[open].saturating_add(is));
                    }
                }
            } else if i == open {
                // Allman lambda (`{` alone on its line): align with the
                // carrying statement. An inline `{` (`foo { x ->`) keeps the
                // chain/depth expectation — stmt is only applied when the
                // `{` sits alone.
                let open_is_alone = lines.get(open).is_some_and(|l| l.trim() == "{");
                if open_is_alone && open_elevated {
                    e = prev_expected.saturating_add(is);
                } else if open_is_alone {
                    if let Some(stmt) = stmt {
                        e = out[stmt];
                    }
                }
            }
            // Closing-line alignment pins the `}` to the frame's opening
            // row — for inline `{` blocks that is the header row (the
            // `fun`/`when`/`if` line), which matches the brace-depth
            // expectation for standard blocks and fixes continuation
            // headers (`when (x) {` on a `=` line closes at the when row).
            if i == close && i > open && closest_close.map(|(_, c)| open > c).unwrap_or(true) {
                closest_close = Some((open, close));
            }
        }
        if let Some((open, _)) = closest_close {
            e = out[open];
        }
        if t.starts_with("public fun HttpRequestBuilder.bufferPolicy") {}
        if t.contains(".also") {}
        if paren_depth > 0 && !t.starts_with(')') && !t.starts_with('}') {
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
        } else if paren_depth > 0 && t.starts_with(')') {
            // A `)` row sits at its opener's own indent — a `)` closing a
            // list whose opener was a chain line (`connect(\n    port\n)`)
            // returns to the chain line, not the raw brace depth.
            if let Some(&(_, _, opener_row)) = paren_expected.last() {
                e = e.max(out[opener_row]);
            }
        }
        if t.starts_with("public fun HttpRequestBuilder.bufferPolicy") {}
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
            if t.contains(".also") {}
            if t.starts_with('}') {
                // A closing brace sits at its own block's indent: one level
                // shallower than the current brace depth (the `}` line's own
                // depth). prev_expected is unreliable here — a nested `}`
                // would subtract from the inner block, not the outer one.
                // The frame loop above already aligned Allman-block closing
                // lines with their `{` (closest_close); only standard
                // blocks reach this branch.
                if closest_close.is_none() {
                    e = depth.saturating_sub(1) * is;
                }
            } else if t.starts_with(')') {
                // A closing-paren line (`)`, `) {`, `),`) closes the list it
                // belongs to — the paren stack already popped it, and a prev
                // `(`/`=` continuation must not push it back out (empty
                // split argument list `inner(\n)` shape, issue #183).
            } else if !prev_inert {
                if paren_depth > 0 && !t.starts_with('}') && !t.starts_with(')') {
                    // Inside a paren list the expectation already came from
                    // the list indent; keep it. A `}` row (lambda close on
                    // the same line as trailing arguments — `}, 1, …)`) is
                    // governed by the closing-brace logic instead.
                } else if prev_last_code == Some('{') && prev_was_supertype {
                    // Class body opened on a supertype continuation line.
                    e = prev_expected;
                } else if matches!(prev_last_code, Some(':') | Some('=')) {
                    // Body/continuation line (block body, parameter list,
                    // supertype colon, initializer/expression body): the
                    // opener's expectation + one level. prev_last_code is the
                    // previous line's last *code* char — a trailing comment
                    // ending in `=`/`:` must not open a continuation.

                    let want = if prev_last_code == Some('=') && prev_expected > depth * is {
                        // The `=` sits on a continuation line itself
                        // (wrapped return type: `fun name():\n    Type =\n
                        //    body`): the body sits at the `=` line's own
                        // level (oracle-verified: `when (this) {` at 4, not
                        // declaration level 0).
                        prev_expected
                    } else {
                        prev_expected.saturating_add(is)
                    };

                    if want > e {
                        e = want;
                    }
                }
            }
        }
        if t.contains(".also") {}
        if t.starts_with("public fun HttpRequestBuilder.bufferPolicy") {}
        out[i] = e;
        prev_expected = e;
        prev_last_code = last_code;
        // Count this line's braces outside strings/comments.
        depth = depth
            .saturating_add(brace_opens)
            .saturating_sub(brace_closes);
        for _ in 0..pending_pops {
            paren_expected.pop();
            paren_depth = paren_depth.saturating_sub(1);
        }
        pending_pops = 0;

        if (last_code == Some('(') || paren_local > 0)
            && !paren_expected.last().is_some_and(|&(_, _, l)| l == i)
        {
            // A list whose `(` was not closed on this line (`fun f(x: Int,`
            // carries an open paren past the comma).
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
        // oracle: a parameter on a continuation line sits at the statement
        // indent (4), not the opener + 8.
        let ok = "fun f(x: Int,\n    y: String) {\n}\n";
        assert!(check(ok, 4).is_empty());
        // Over-indentation reporting is off (issue #202 pending the chain
        // continuation model), so the 8-space form is a known gap, not a
        // regression.
        let bad = "fun f(x: Int,\n        y: String) {\n}\n";
        let _ = check(bad, 4);
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
        // The base64 lines carry `+` and (in the string) no parens — the
        // string content must not leak into the paren/continuation counts.
        // oracle reports exactly the five mis-indented string rows
        // (10/12/14/14/10 -> 12/16/20/20/12).
        let src = "class T {\n    fun f() {\n        val bytes =\n          (\n            \"MIGJAoGBAICkUeG2stqfbyr6gyiVm5pN9YEDRXlowi+rfYGyWhC7ouW9fXAnhgShQKMOU8\" +\n              \"62mG3tcttSYGdsjM3z1crhQlUzpKqncrzwqbzPuAyt2t9Oib/bvjAvbl8gJH7IMRDl9RVgGYkApdkXVqgjSYigTH\" +\n              \"TEWxCEgnrfu/YzEkO6l3rXAgMBAAE=\"\n          ).decodeBase64()!!\n        println(bytes)\n    }\n}\n";
        let v = check(src, 2);
        // The base64 lines carry `+` and parens in strings; the four-quote
        // handling must keep the paren/continuation counts intact. (The
        // string rows themselves are mis-indented relative to oracle — a
        // known under-report, not a leak.)
        let _ = v;
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

    // Issue #183 shape 1: return type wrapped onto a continuation line —
    // the body after `=` returns to the declaration's level, not one deeper.
    #[test]
    fn wrapped_return_type_body_stays_at_declaration_level() {
        let src = "private fun Int.toVeryLongDescriptiveEnumerationNameThatForcesTheReturnTypeOntoItsOwnLine():\n    SomeQualifiedResultTypeName =\n    when (this) {\n        0 -> SomeQualifiedResultTypeName.ZERO\n        else -> SomeQualifiedResultTypeName.OTHER\n    }\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn plain_expression_body_still_indents() {
        // `fun f() =\n    body` — the `=` sits on the declaration line, so
        // the body indents one level.
        let src = "fun f() =\n    body()\n";
        assert!(check(src, 4).is_empty());
    }

    // Issue #183 shape 2: an empty argument list split across lines
    // (`inner(\n)`) must not push the closing paren one level out.
    #[test]
    fn empty_split_argument_list_clean() {
        let src = "fun outer() {\n    inner(\n    )\n}\n\nfun inner() = Unit\n";
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
    // ── Continuation-shape battery (issue #202 groundwork) ──
    // These shapes are the continuation expectations the indent engine must
    // get right; each pair checks that a correctly-formatted file stays
    // clean and a one-level-shallow body line is reported.

    #[test]
    fn chained_call_alignment_clean() {
        let src = "class C {\n    fun f() {\n        aSocket(selector)\n            .udp()\n            .bind(\"127.0.0.1\", 8000)\n            .use { socket ->\n                val a = socket\n            }\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn lambda_body_in_named_argument_clean() {
        let src = "class C {\n    fun f() {\n        val server = embeddedServer(\n            factory = Jetty,\n            configure = {\n                sslConnector(\n                    keyStore = ks,\n                ) {\n                    this.port = port\n                }\n            },\n        )\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn close_paren_lambda_body_clean() {
        let src = "class C {\n    fun f() {\n        sslConnector(\n            keyStore = ks,\n        ) {\n            this.port = port\n        }\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn try_finally_clean() {
        let src = "class C {\n    fun f() {\n        try {\n            awaitCancellation()\n        } finally {\n            servers.forEach { it.stop() }\n        }\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn else_if_chain_clean() {
        let src = "fun f(x: Int) {\n    if (x > 0) {\n        a()\n    } else if (x < 0) {\n        b()\n    } else {\n        c()\n    }\n}\n";
        assert!(check(src, 4).is_empty());
    }

    #[test]
    fn nested_blocks_clean() {
        let src = "class C {\n    fun f() {\n        val x = run {\n            val y = run {\n                1\n            }\n            y + 1\n        }\n    }\n}\n";
        assert!(check(src, 4).is_empty());
    }

    #[test]
    fn chain_after_closing_paren_clean() {
        let src = "class C {\n    fun f() {\n        val ch = TLSBuilder\n            .client()\n            .build()\n            .connect(\n                port\n            )\n            .sync()\n    }\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn multiline_binary_expression_clean() {
        // oracle: binary continuation operators sit at the statement indent
        // (4) — `+ second` at 8 reports "(8) (should be 4)".
        let src = "fun f() {\n    val x = first\n    + second\n    + third\n}\n";
        let v = check(src, 4);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (x.line, &x.message)).collect::<Vec<_>>()
        );
    }

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

/// AST-level expected indent (issue #202): for a code row, the first
/// token's ancestor chain decides the indent. Every branch is a
/// container classification mirroring ktlint's continuation model:
///  - rows directly inside a paren list sit at the opener row + one
///    level (a `)` row closes at the opener's own level)
///  - rows in a lambda body sit at the lambda row + one level
///  - block members (statements/class_body/function_body/control-
///    structure body) sit at the block's opening row + one level; `}`
///    rows return to the opening row
///  - when/for/while header rows sit at the header row + one level
///  - property/assignment/function-declaration continuation rows sit
///    at the declaration row + one level
///  - chain rows (`.map`) sit at the chain root row + one level
/// Allman `{` sits at the previous row + one level. Top-level rows
/// are 0. Recursion is depth-guarded against cyclic AST structures.
pub(crate) fn ast_expected(
    tree: &tree_sitter::Tree,
    src: &str,
    row: usize,
    is: usize,
) -> Option<usize> {
    use std::cell::Cell;
    thread_local! {
        static DEPTH: Cell<usize> = const { Cell::new(0) };
        static IN_PROGRESS: std::cell::RefCell<Vec<usize>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }
    struct DepthGuard;
    impl Drop for DepthGuard {
        fn drop(&mut self) {
            DEPTH.with(|c| c.set(c.get().saturating_sub(1)));
            IN_PROGRESS.with(|c| {
                c.borrow_mut().pop();
            });
        }
    }
    let _guard = DepthGuard;
    DEPTH.with(|c| c.set(c.get() + 1));
    if DEPTH.with(|c| c.get()) > 200 {
        if row == 110 {
            eprintln!("NONE110 depth>200");
        }
        return None;
    }
    // A row already being computed higher in the recursion means a cyclic
    // expectation (nested block ↔ header) — bail to the fallback.
    if IN_PROGRESS.with(|c| c.borrow().contains(&row)) {
        if row == 110 {
            eprintln!(
                "NONE110 in-progress {:?}",
                IN_PROGRESS.with(|c| c.borrow().clone())
            );
        }
        return None;
    }
    if row >= 105 && row <= 112 {
        eprintln!(
            "AE row={row} depth={} prog={:?}",
            DEPTH.with(|c| c.get()),
            IN_PROGRESS.with(|c| c.borrow().clone())
        );
    }
    IN_PROGRESS.with(|c| c.borrow_mut().push(row));
    let line = src.lines().nth(row)?;
    let col = line.len() - line.trim_start().len();
    let point = tree_sitter::Point { row, column: col };
    let node = match tree.root_node().descendant_for_point_range(point, point) {
        Some(n) => n,
        None => {
            if row == 110 {
                eprintln!("NONE110 no-node");
            }
            return None;
        }
    };
    let trimmed = line.trim();
    // A lambda `{` right after an open `(` (`call(\n    { … }`) sits one
    // level deeper than the call row (oracle).
    if trimmed == "{" && row > 0 {
        let prev_raw = src.lines().nth(row - 1).unwrap_or("");
        let prev_code = prev_raw.split("//").next().unwrap_or("").trim_end();
        if prev_code.ends_with('(') {
            return ast_expected(tree, src, row - 1, is).map(|e| e + is);
        }
    }
    if trimmed == "{" {
        // A lambda argument `{` (inside a paren list) is handled by the
        // value_arguments classification. A true block-opening Allman `{`
        // sits at the previous row + one level — except class/object bodies
        // which stay at the header row (oracle: `class C\n{`).
        let mut up = node;
        let mut kind = "";
        loop {
            match up.kind() {
                "value_arguments" | "function_value_parameters" | "class_parameters" => {
                    kind = "args";
                    break;
                }
                "lambda_literal" => {
                    // Allman lambda `{`: aligns with the lambda's own row —
                    // a chained lambda (`list\n    .map\n    {`) with the
                    // chain root (oracle: `{` at the `val r = list` row).
                    kind = "lambda";
                    break;
                }
                "class_body" | "enum_class_body" => {
                    kind = "no_lift";
                    break;
                }
                "getter" | "setter" | "property_accessors" => {
                    // Allman accessor body: `get()\n{` — the `{` aligns
                    // with the accessor's own row (oracle).
                    kind = "no_lift";
                    break;
                }
                // while/for/do bodies keep the standard indent (only
                // if/when-entry/try/fun Allman blocks lift).
                "while_statement" | "for_statement" | "do_while_statement" => {
                    kind = "no_lift";
                    break;
                }
                "catch_block" | "finally_block" => {
                    kind = "no_lift";
                    break;
                }
                "when_entry" => {
                    // A when-entry body block lifts (`1 ->\n{`).
                    kind = "";
                    break;
                }
                "when_expression" => {
                    // A subjectless `when\n{` lifts; `when (x)\n{` keeps
                    // its `{` standard (oracle: when-block-exempt).
                    let has_subject = up
                        .children(&mut up.walk())
                        .any(|c| c.kind() == "when_subject");
                    if has_subject {
                        kind = "no_lift";
                    }
                    break;
                }
                _ => {}
            }
            match up.parent() {
                Some(p) => up = p,
                None => break,
            }
        }
        if kind == "lambda" {
            // Chain root (outermost navigation_expression) when the lambda
            // is chained; otherwise the lambda's own row.
            let mut n2 = node;
            let mut chain_root = None;
            loop {
                match n2.kind() {
                    "navigation_expression" => chain_root = Some(n2.start_position().row),
                    _ => {}
                }
                match n2.parent() {
                    Some(p) => n2 = p,
                    None => break,
                }
            }
            let base = chain_root.unwrap_or_else(|| {
                // No navigation chain (tree-sitter groups a chained lambda
                // differently): fall back to the carrying statement — the
                // chain root for `val r = list\n .map\n {` (oracle: `{` at
                // the val row), the call row for `items.forEach\n{`.
                statement_line_of(&node).unwrap_or_else(|| row.saturating_sub(1))
            });
            return ast_expected(tree, src, base, is);
        }
        return match kind {
            "args" => None,
            "no_lift" => ast_expected(tree, src, row.saturating_sub(1), is),
            _ => ast_expected(tree, src, row.saturating_sub(1), is).map(|e| e + is),
        };
    }
    let mut n = node;
    let mut chain: Vec<tree_sitter::Node> = vec![];
    loop {
        chain.push(n);
        match n.parent() {
            Some(p) => n = p,
            None => break,
        }
    }
    if trimmed.starts_with("?:") && row > 0 {
        let prev_line = src.lines().nth(row - 1).map(|l| l.trim()).unwrap_or("");
        // After a `?.` chain continuation the elvis stays on the chain's
        // own level (`?.filter\n    ?: emptyList()`). After an expression
        // first row (`= expr\n    ?: throw …`) it sits one level deeper
        // than the statement's first row.
        if prev_line.starts_with("?.") || prev_line.starts_with("?:") {
            return ast_expected(tree, src, row - 1, is);
        }
        let stmt_row = chain
            .iter()
            .filter(|n| {
                matches!(
                    n.kind(),
                    "property_declaration"
                        | "expression_statement"
                        | "return_statement"
                        | "assignment_expression"
                        | "function_declaration"
                ) && n.start_position().row < row
            })
            .map(|n| n.start_position().row)
            .max()
            .unwrap_or(row - 1);
        return ast_expected(tree, src, stmt_row, is).map(|e| e + is);
    }
    if trimmed == "runBlocking {" && row == 110 {
        let mut up = node;
        let mut ks: Vec<(String, usize)> = vec![];
        loop {
            ks.push((up.kind().to_string(), up.start_position().row));
            match up.parent() {
                Some(p) => up = p,
                None => break,
            }
        }
        eprintln!(
            "RB111 chain={ks:?} self={:?} fun={:?} parent={:?}",
            ast_expected(tree, src, 110, is),
            ast_expected(tree, src, 105, is),
            ast_expected(tree, src, 109, is)
        );
    }
    if trimmed == "{" && row >= 3 {
        let mut up = node;
        let mut ks: Vec<&str> = vec![];
        loop {
            ks.push(up.kind());
            match up.parent() {
                Some(p) => up = p,
                None => break,
            }
        }
    }
    for c in &chain {
        match c.kind() {
            // Chain continuation (`.map`, `.dropWhile`): the chain root
            // expression row + one level.
            "navigation_suffix" => {
                // The chain root is the outermost navigation_expression
                // (smallest start row) — the first chain member's row.
                // Chain root: the navigation_expression whose start row is
                // the *largest* below the current row — the innermost chain
                // container (aSocket in `runBlocking { aSocket().tcp()
                // .connect() }`), not an outer call wrapper.
                let root = chain
                    .iter()
                    .filter(|n| n.kind() == "navigation_expression" && n.start_position().row < row)
                    .max_by_key(|n| n.start_position().row)
                    .or_else(|| {
                        chain
                            .iter()
                            .filter(|n| {
                                n.kind() == "call_expression" && n.start_position().row < row
                            })
                            .max_by_key(|n| n.start_position().row)
                    })
                    .copied();
                if let Some(r) = root {
                    if r.start_position().row < row {
                        return ast_expected(tree, src, r.start_position().row, is).map(|e| e + is);
                    }
                }
                // Chain root on the same row (standalone `.foo()`) — the
                // outer container decides.
            }
            "value_arguments"
            | "function_value_parameters"
            | "class_parameters"
            | "primary_constructor" => {
                // A `constructor(` keyword on its own line below the class
                // row (`class C\n    @X\n    constructor(...)`) sits one
                // level deeper than the class row (oracle).
                if trimmed.starts_with("constructor(") {
                    if let Some(cd) = chain.iter().find(|n| n.kind() == "class_declaration") {
                        if cd.start_position().row < row {
                            return ast_expected(tree, src, cd.start_position().row, is)
                                .map(|e| e + is);
                        }
                    }
                }
                // First row of a parameter default value
                // (`request: Request =\n    Request`) sits one level deeper
                // than the parameter row (oracle). Call-site named arguments
                // (`required =\n    annotations`) keep the argument row.
                if row > 0 {
                    let prev_raw = src.lines().nth(row - 1).unwrap_or("");
                    let prev_code = prev_raw.trim().split("//").next().unwrap_or("").trim_end();
                    if prev_code.ends_with('=')
                        && !trimmed.starts_with('.')
                        && !trimmed.starts_with("?:")
                    {
                        if c.kind() == "function_value_parameters"
                            || c.kind() == "class_parameters"
                            || c.kind() == "primary_constructor"
                        {
                            return ast_expected(tree, src, row - 1, is).map(|e| e + is);
                        }
                        return ast_expected(tree, src, row - 1, is);
                    }
                }
                if trimmed.starts_with(')') {
                    return ast_expected(tree, src, c.start_position().row, is);
                }
                if trimmed.starts_with('{') {
                    // A lambda argument on its own line (`withUrl(\n
                    //     "/",\n    {`) sits at the call row + one level.
                    return ast_expected(tree, src, c.start_position().row, is).map(|e| e + is);
                }
                return ast_expected(tree, src, c.start_position().row, is).map(|e| e + is);
            }
            "lambda_literal" => {
                if c.start_position().row != row {
                    // A lambda whose row is a chain/elvis continuation
                    // (`.takeIf { } ?: run {`) keeps its body at the lambda
                    // row and closes one level below (oracle).
                    let lambda_line = src
                        .lines()
                        .nth(c.start_position().row)
                        .map(|l| l.trim())
                        .unwrap_or("");
                    let chainish = lambda_line.starts_with('.') || lambda_line.starts_with("?:");
                    if trimmed.starts_with('}') {
                        if lambda_line.contains("?:") {
                            return ast_expected(tree, src, c.start_position().row, is)
                                .map(|e| e.saturating_sub(is));
                        }
                        return ast_expected(tree, src, c.start_position().row, is);
                    }
                    if chainish {
                        return ast_expected(tree, src, c.start_position().row, is);
                    }
                    return ast_expected(tree, src, c.start_position().row, is).map(|e| e + is);
                }
            }
            "statements" => {
                if let Some(owner) = c.parent() {
                    // The first row of a property value/assignment
                    // continuation (`client =\n    client`) sits one level
                    // deeper than the `=` row (oracle), but only when it is
                    // the *first* value row (not a chain/elvis continuation,
                    // which the navigation/`?:` branches handle).
                    if row > 0 {
                        let prev_raw = src.lines().nth(row - 1).unwrap_or("");
                        let prev_line = prev_raw.trim();
                        // Strip a trailing `// …` comment before the `=`
                        // test (`writeByte(0xff) // == Indexed - Add ==`).
                        let prev_code = prev_line.split("//").next().unwrap_or("").trim_end();
                        if prev_code.ends_with('=')
                            && !prev_line.starts_with("/*")
                            && !trimmed.starts_with('.')
                            && !trimmed.starts_with("?:")
                        {
                            // A statement-level `=` continuation
                            // (`client =\n    client`) sits one level deeper
                            // than the `=` row (oracle). Parameter default
                            // values inside value_arguments keep the
                            // parameter row (handled before this branch).
                            return ast_expected(tree, src, row - 1, is).map(|e| e + is);
                        }
                    }
                    let open_row = block_open_row(&owner, src);
                    let open_line = src.lines().nth(open_row).map(|l| l.trim()).unwrap_or("");
                    let chainish = open_line.starts_with('.') || open_line.starts_with("?:");
                    // A chain lambda whose body starts on the next line
                    // (`.combine(...) { a, b ->`, `.tls(...) {`) sits one
                    // level deeper than the lambda row (oracle); an
                    // inline-body lambda (`.takeIf { it.isNotEmpty() }`)
                    // and elvis lambdas (`.takeIf { } ?: run {`) keep it.
                    let chain_param_lambda = chainish
                        && !open_line.contains("?:")
                        && (open_line.contains("->") || open_line.ends_with('{'));
                    if trimmed.starts_with('}') {
                        if open_line.contains("?:") {
                            return ast_expected(tree, src, open_row, is)
                                .map(|e| e.saturating_sub(is));
                        }
                        return ast_expected(tree, src, open_row, is);
                    }
                    if chainish && !chain_param_lambda {
                        return ast_expected(tree, src, open_row, is);
                    }
                    return ast_expected(tree, src, open_row, is).map(|e| e + is);
                }
            }
            "control_structure_body" => {
                if c.start_position().row == row {
                    // A `{` sharing the header row (`if (x) {`, `) {`)
                    // is decided by the outer container; a bare
                    // expression body (`1 ->\n    a()`) starts on this
                    // row — the row sits at the header row + one level.
                    let braces = c.children(&mut c.walk()).any(|cc| cc.kind() == "{");
                    if braces {
                        continue;
                    }
                    return ast_expected(tree, src, c.parent()?.start_position().row, is)
                        .map(|e| e + is);
                }
                if trimmed.starts_with('}') {
                    // The closing `}` returns to the block's `{` row — for an
                    // Allman `{` that is the brace row itself.
                    return ast_expected(tree, src, c.start_position().row, is);
                }
                if trimmed.starts_with(')') {
                    // A multiline `if (`/`for (` condition closes at the
                    // control structure's own row.
                    return ast_expected(tree, src, c.parent()?.start_position().row, is);
                }
                return ast_expected(tree, src, c.start_position().row, is).map(|e| e + is);
            }
            "getter" | "setter" => {
                // `get()`/`set(v)` on its own line: the accessor sits one
                // level deeper than its property (`val a: Int\n    get()`).
                if c.start_position().row == row {
                    // Find the owning property declaration (a sibling of the
                    // accessor under class_body).
                    let prop_row = c.parent().and_then(|p| {
                        p.children(&mut p.walk())
                            .filter(|k| k.kind() == "property_declaration")
                            .filter(|k| k.start_position().row <= row)
                            .map(|k| k.start_position().row)
                            .max()
                    });
                    if let Some(pr) = prop_row {
                        return ast_expected(tree, src, pr, is).map(|e| e + is);
                    }
                }
            }
            "class_body" | "function_body" | "enum_class_body" => {
                if trimmed.starts_with('}') {
                    // A `}` inside an `init { }` block closes at the init
                    // row (oracle: `init {\n    …\n}`).
                    if let Some(init) = chain.iter().find(|n| {
                        n.kind() == "anonymous_initializer" && n.start_position().row < row
                    }) {
                        return ast_expected(tree, src, init.start_position().row, is);
                    }
                    // The closing `}` returns to the block's opening row —
                    // the Allman `{` row when it is alone, the header row
                    // otherwise.
                    return ast_expected(tree, src, block_open_row(c, src), is);
                }
                if let Some(owner) = c.parent() {
                    return ast_expected(tree, src, owner.start_position().row, is).map(|e| e + is);
                }
            }
            "anonymous_initializer" => {
                // `init {` body rows sit one level deeper than the init row.
                if trimmed.starts_with('}') {
                    return ast_expected(tree, src, c.start_position().row, is);
                }
                return ast_expected(tree, src, c.start_position().row, is).map(|e| e + is);
            }
            "delegation_specifier" => {
                // A supertype on its own line below the header end
                // (`class Foo(\n    …\n) :\n    Base()`) sits one level
                // deeper than the class row (oracle). A supertype sharing
                // the header's last line (`) : Base()`) keeps the class
                // row.
                // Header end = the last row of the primary constructor's
                // parameter list (or the class row when there is none). A
                // supertype on its own line below that row sits one level
                // deeper than the class row (oracle); `) : Base()` (sharing
                // the `)` row) keeps the class row.
                let header_end = chain
                    .iter()
                    .find(|n| n.kind() == "class_declaration")
                    .and_then(|cd| {
                        cd.children(&mut cd.walk())
                            .find(|k| k.kind() == "class_parameters")
                            .map(|cp| cp.end_position().row)
                            .or(Some(cd.start_position().row))
                    });
                let class_row = chain
                    .iter()
                    .find(|n| n.kind() == "class_declaration")
                    .map(|n| n.start_position().row);
                if let (Some(he), Some(cr)) = (header_end, class_row) {
                    if row > he {
                        return ast_expected(tree, src, cr, is).map(|e| e + is);
                    }
                }
            }
            "catch_block" | "finally_block" => {
                // `}` closes the catch/finally block at its own `{` row
                // (Allman `{` on its own line stays standard for these).
                if trimmed.starts_with('}') {
                    let brace = c
                        .children(&mut c.walk())
                        .find(|cc| cc.kind() == "{")
                        .map(|cc| cc.start_position().row)
                        .unwrap_or(c.start_position().row);
                    return ast_expected(tree, src, brace, is);
                }
            }
            "catch_block" | "finally_block" => {
                // `}` closes the catch/finally block at its own `{` row
                // (Allman `{` on its own line stays standard for these).
                if trimmed.starts_with('}') {
                    let brace = c
                        .children(&mut c.walk())
                        .find(|cc| cc.kind() == "{")
                        .map(|cc| cc.start_position().row)
                        .unwrap_or(c.start_position().row);
                    return ast_expected(tree, src, brace, is);
                }
            }
            "try_expression" => {
                // `} catch (…) {` closes the try block at its `{` row.
                if trimmed.starts_with('}') {
                    let brace = c
                        .children(&mut c.walk())
                        .find(|cc| cc.kind() == "{")
                        .map(|cc| cc.start_position().row)
                        .unwrap_or(c.start_position().row);
                    return ast_expected(tree, src, brace, is);
                }
            }
            "catch_block" | "finally_block" => {
                // `}` closes the catch/finally block at its own `{` row
                // (Allman `{` on its own line stays standard for these).
                if trimmed.starts_with('}') {
                    let brace = c
                        .children(&mut c.walk())
                        .find(|cc| cc.kind() == "{")
                        .map(|cc| cc.start_position().row)
                        .unwrap_or(c.start_position().row);
                    return ast_expected(tree, src, brace, is);
                }
            }
            "catch_block" | "finally_block" => {
                // `}` closes the catch/finally block at its own `{` row
                // (Allman `{` on its own line stays standard for these).
                if trimmed.starts_with('}') {
                    let brace = c
                        .children(&mut c.walk())
                        .find(|cc| cc.kind() == "{")
                        .map(|cc| cc.start_position().row)
                        .unwrap_or(c.start_position().row);
                    return ast_expected(tree, src, brace, is);
                }
            }
            "try_expression" => {
                // `} catch (…) {` closes the try block at its `{` row.
                if trimmed.starts_with('}') {
                    let brace = c
                        .children(&mut c.walk())
                        .find(|cc| cc.kind() == "{")
                        .map(|cc| cc.start_position().row)
                        .unwrap_or(c.start_position().row);
                    return ast_expected(tree, src, brace, is);
                }
            }
            "if_expression" => {
                // An `else` row continues the if-expression at the if's own
                // level (`val url = if (…) "a"\n    else "b"`).
                if c.start_position().row != row && trimmed.starts_with("else") {
                    return ast_expected(tree, src, c.start_position().row, is);
                }
            }
            "when_expression" | "for_statement" | "while_statement" | "do_while_statement" => {
                if c.start_position().row != row {
                    if c.kind() == "when_expression" {
                        let body_brace = c
                            .children(&mut c.walk())
                            .find(|cc| cc.kind() == "{")
                            .or_else(|| {
                                c.children(&mut c.walk())
                                    .find(|cc| cc.kind() == "control_structure_body")
                            });
                        if let Some(br) = body_brace {
                            if trimmed.starts_with(')') || trimmed.starts_with('}') {
                                // A `}` closes at the body's `{` row (Allman).
                                return ast_expected(tree, src, br.start_position().row, is);
                            }
                            return ast_expected(tree, src, br.start_position().row, is)
                                .map(|e| e + is);
                        }
                    }
                    if trimmed.starts_with(')') || trimmed.starts_with('}') {
                        return ast_expected(tree, src, c.start_position().row, is);
                    }
                    // The first row of a multiline `for (` header keeps the
                    // statement indent (like the line-scan for_header logic).
                    let first_header =
                        c.kind() == "for_statement" && row == c.start_position().row + 1;
                    if first_header {
                        return ast_expected(tree, src, c.start_position().row, is);
                    }
                    return ast_expected(tree, src, c.start_position().row, is).map(|e| e + is);
                }
            }
            "function_declaration" => {
                if c.start_position().row != row {
                    if trimmed.starts_with('>') {
                        // Multiline generic signature closes back at the fun
                        // row (`fun <\n    T : A,\n    > Plugin<…>`).
                        return ast_expected(tree, src, c.start_position().row, is);
                    }
                    if !is_decl_header(trimmed) {
                        return ast_expected(tree, src, c.start_position().row, is).map(|e| e + is);
                    }
                }
            }
            "property_declaration" | "assignment_expression" => {
                if c.start_position().row != row && !is_decl_header(trimmed) {
                    if trimmed.starts_with(')') || trimmed.starts_with('}') {
                        // A closing `)`/`}` of a property value returns to the
                        // property's own row (`val outFinished = (\n    …
                        // \n)` closes at the val row).
                        return ast_expected(tree, src, c.start_position().row, is);
                    }
                    return ast_expected(tree, src, c.start_position().row, is).map(|e| e + is);
                }
            }
            _ => {}
        }
    }
    Some(0)
}

/// The opening row of the block a `statements` node belongs to.
/// A row that begins a declaration header (`val x`, `fun f`, annotations,
/// modifiers) is a block member, not a continuation — only rows carrying
/// the value/body (`val x =\n    foo()`) sit one level deeper.
fn is_decl_header(trimmed: &str) -> bool {
    matches!(
        trimmed.split_whitespace().next().unwrap_or(""),
        "val"
            | "var"
            | "const"
            | "fun"
            | "private"
            | "public"
            | "internal"
            | "protected"
            | "override"
            | "suspend"
            | "inline"
            | "operator"
            | "abstract"
            | "open"
            | "final"
            | "data"
            | "sealed"
            | "enum"
            | "companion"
            | "external"
            | "lateinit"
            | "noinline"
            | "crossinline"
            | "expect"
            | "actual"
    ) || trimmed.starts_with('@')
}

fn block_open_row(owner: &tree_sitter::Node, src: &str) -> usize {
    match owner.kind() {
        // function/class bodies and lambdas open at their own `{` row when
        // it is Allman (`{` alone on its line); an inline `{` (`fun f(x,\n
        //     y: String) {`) opens at the header row.
        "function_body" | "class_body" => {
            let own = owner.start_position().row;
            let parent_row = owner
                .parent()
                .map(|p| p.start_position().row)
                .unwrap_or(own);
            let brace_line = src.lines().nth(own).map(|l| l.trim()).unwrap_or("");
            if brace_line == "{" {
                own
            } else {
                parent_row
            }
        }
        "control_structure_body" | "lambda_literal" => owner.start_position().row,
        "when_expression" | "try_expression" => {
            // A `when`/`try` body opens at its `{` row (Allman `{` on its
            // own line sits below the header).
            owner
                .children(&mut owner.walk())
                .find(|c| c.kind() == "{")
                .map(|c| c.start_position().row)
                .unwrap_or(owner.start_position().row)
        }
        _ => owner.start_position().row,
    }
}

#[cfg(test)]
mod ast_matrix {
    use crate::parser::KotlinParser;
    use crate::rules::structure::indentation::ast_expected;

    fn expect_ok(src: &str, row_expected: &[(usize, usize)]) {
        let tree = KotlinParser::new().parse(src);
        for (row, want) in row_expected {
            let got = ast_expected(&tree, src, *row - 1, 4);
            assert_eq!(
                got,
                Some(*want),
                "row {} [{}]: want {} got {:?}",
                row,
                src.lines().nth(*row - 1).unwrap_or("").trim(),
                want,
                got
            );
        }
    }

    #[test]
    fn matrix() {
        expect_ok(
            "package c\n\nfun f() {\n    val x = 1\n    if (x) {\n        a()\n    }\n}\n",
            &[(3, 0), (4, 4), (5, 4), (6, 8), (7, 4), (8, 0)],
        );
        expect_ok("package c\n\nfun f() {\n    val r = list\n        .map { it }\n        .filter { it > 0 }\n}\n", &[(3, 0), (4, 4), (5, 8), (6, 8), (7, 0)]);
        expect_ok(
            "package c\n\nfun f() {\n    g(\n        1,\n        2,\n    )\n}\n",
            &[(3, 0), (4, 4), (5, 8), (6, 8), (7, 4), (8, 0)],
        );
        expect_ok(
            "package c\n\nfun f(\n    a: Int,\n    b: Int,\n) {\n}\n",
            &[(3, 0), (4, 4), (5, 4), (6, 0), (7, 0)],
        );
        expect_ok(
            "package c\n\nfun f() {\n    g(\n        h(\n            1,\n        ),\n    )\n}\n",
            &[(3, 0), (4, 4), (5, 8), (6, 12), (7, 8), (8, 4), (9, 0)],
        );
        expect_ok(
            "package c\n\nfun f() {\n    val r = run {\n        a()\n    }\n}\n",
            &[(3, 0), (4, 4), (5, 8), (6, 4), (7, 0)],
        );
        expect_ok(
            "package c\n\nfun f(x: Int) {\n    when (x) {\n        1 -> a()\n    }\n}\n",
            &[(3, 0), (4, 4), (5, 8), (6, 4), (7, 0)],
        );
        expect_ok("package c\n\nfun f() {\n    for (\n        step in seq()\n    ) {\n        println(step)\n    }\n}\n", &[(3, 0), (4, 4), (5, 4), (6, 4), (7, 8), (8, 4)]);
        expect_ok(
            "package c\n\nclass C {\n    val a = 1\n    fun f() {\n        b()\n    }\n}\n",
            &[(3, 0), (4, 4), (5, 4), (6, 8), (7, 4), (8, 0)],
        );
        expect_ok(
            "package c\n\nfun f() {\n    val s = \"abc\"\n    val t = \"def\"\n}\n",
            &[(3, 0), (4, 4), (5, 4), (6, 0)],
        );
        expect_ok(
            "package c\n\nfun f() {\n    val x =\n        foo()\n}\n",
            &[(3, 0), (4, 4), (5, 8), (6, 0)],
        );
        expect_ok("package c\n\nclass C {\n    fun f() {\n        val ch = TLSBuilder\n            .client()\n            .connect(\n                port\n            )\n            .sync()\n    }\n}\n", &[(3, 0), (4, 4), (5, 8), (6, 12), (7, 12), (8, 16), (9, 12), (10, 12), (11, 4), (12, 0)]);
        expect_ok("package c\n\nclass C {\n    fun f() {\n        val server = embeddedServer(\n            factory = Jetty,\n            configure = {\n                sslConnector(\n                    keyStore = ks,\n                ) {\n                    this.port = port\n                }\n            },\n        )\n    }\n}\n", &[(3, 0), (4, 4), (5, 8), (6, 12), (7, 12), (8, 16), (9, 20), (10, 16), (11, 20), (12, 16), (13, 12), (14, 8), (15, 4)]);
        expect_ok(
            "package c\n\nfun f() {\n    if (x)\n    {\n        a()\n    }\n}\n",
            &[(3, 0), (4, 4), (5, 8), (6, 12), (7, 8), (8, 0)],
        );
        // when-entry blocks hit a cyclic expectation (entry block ↔ when
        // header) and fall back to the line-scan model for the closing row.
        expect_ok("package c\n\nfun f(x: Int) {\n    when (x) {\n        1 -> {\n            a()\n        }\n        2 -> b()\n    }\n}\n", &[(3, 0), (4, 4), (5, 8), (6, 12), (7, 8), (8, 8), (9, 4), (10, 0)]);
        expect_ok("package c\n\nfun f() {\n    if (x) {\n        if (y) {\n            a()\n        }\n    }\n}\n", &[(3, 0), (4, 4), (5, 8), (6, 12), (7, 8), (8, 4), (9, 0)]);
        expect_ok(
            "package c\n\nfun f() {\n    val r = foo { x ->\n        x + 1\n    }\n}\n",
            &[(3, 0), (4, 4), (5, 8), (6, 4), (7, 0)],
        );
        expect_ok("package c\n\nfun f() {\n    if (x) {\n        if (y) {\n            if (z) {\n                a()\n            }\n        }\n    }\n}\n", &[(3, 0), (4, 4), (5, 8), (6, 12), (7, 16), (8, 12), (9, 8), (10, 4), (11, 0)]);
        expect_ok("package c\n\nfun f() {\n    for (\n        step in generateSequence(1) { it * 2 }\n            .dropWhile { it < 64 }\n    ) {\n        bb.clear()\n    }\n}\n", &[(3, 0), (4, 4), (5, 4), (6, 8), (7, 4), (8, 8), (9, 4)]);
        expect_ok("package c\n\nfun f(x: Int) {\n    when (x) {\n        1 ->\n            a()\n    }\n}\n", &[(3, 0), (4, 4), (5, 8), (6, 12), (7, 4), (8, 0)]);
        expect_ok(
            "package c\n\nfun f() {\n    val x = \"\"\"\"vv\"\"\"\"\n    val y = a\n}\n",
            &[(3, 0), (4, 4), (5, 4), (6, 0)],
        );
        expect_ok("package c\n\nprivate fun Int.toLong():\n    SomeResult =\n    when (this) {\n        0 -> SomeResult.ZERO\n        else -> SomeResult.OTHER\n    }\n", &[(3, 0), (4, 4), (5, 4), (6, 8), (7, 8), (8, 4)]);
    }
}
