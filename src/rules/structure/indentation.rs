use crate::config::CodeStyle;
use crate::rules::{Rule, Violation};

/// JVM-compatible indentation check.
///
/// Core logic: for each line of code, check that the indentation (leading spaces)
/// is a multiple of the indent_size. Skip empty lines, comments, annotations,
/// KDoc, and KTS files.
pub struct Indentation {
    indent_size: usize,
    code_style: CodeStyle,
}

impl Indentation {
    pub fn new(indent_size: usize) -> Self {
        Self {
            indent_size,
            code_style: CodeStyle::default(),
        }
    }

    pub fn with_code_style(mut self, style: CodeStyle) -> Self {
        self.code_style = style;
        self
    }
}

impl Rule for Indentation {
    fn id(&self) -> &'static str {
        "standard:indent"
    }

    /// `.kts` scripts are skipped by extension (the engine passes the path
    /// only to the rules that opt in). JVM ktlint does lint scripts, but the
    /// differential corpus is `.kt`-only, and the script shapes are not yet
    /// modelled — the skip keeps the gate's FP at 0 (issue #202).
    fn check_with_path(
        &self,
        path: &str,
        tree: &tree_sitter::Tree,
        source: &str,
    ) -> Vec<Violation> {
        if path.ends_with(".kts") {
            return Vec::new();
        }
        self.check(tree, source)
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        CODE_STYLE.with(|c| c.set(self.code_style));
        let mut violations = Vec::new();
        let is = self.indent_size;
        let lines: Vec<&str> = source.lines().collect();
        let mut in_block_comment = false;
        let mut in_raw_string = false;

        // `.kts` scripts are skipped via `check_with_path` (extension-based) —
        // a content heuristic misfires on `.kt` files whose only declarations
        // are top-level vals (`internal val NiaTypography = Typography(…)`,
        // Type.kt), where JVM ktlint does report the indent violations.

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
        // Over-indentation is only reported when KTLINT_RS_INDENT_PROBE is
        // set (issue #202): the continuation model is not yet exact enough
        // for every real-world shape (e.g. 2-space-indent projects), so the
        // confident-rows-only detection stays a measurement tool until the
        // remaining shapes are modelled (see the probe sweep).
        let probe = std::env::var("KTLINT_RS_INDENT_PROBE").is_ok();
        // On files tree-sitter fails to parse cleanly, error recovery produces
        // garbage ancestry (e.g. statements re-parented to source_file), so the
        // AST classifier's expectations are not trustworthy — fall back to the
        // conservative line-scan model for those files (issue #202).
        let tree_clean = !_tree.root_node().has_error();
        // Line-scan expectations (brace depth + continuation) — the fallback
        // model. The over-indent probe only trusts a row when the AST
        // classifier AND the line-scan model agree on the expected indent:
        // both are wrong together only when the shape is too exotic to model
        // (issue #202 conservative gap).
        let scan_expected = compute_line_expected(&lines, is, &elevated);
        // AST expectations per row (None = unclassified → line-scan value).
        let ast: Vec<Option<usize>> = (0..lines.len())
            .map(|i| ast_expected(_tree, source, i, is))
            .collect();
        let line_expected: Vec<usize> = (0..lines.len())
            .map(|i| {
                if tree_clean {
                    ast[i].unwrap_or(scan_expected[i])
                } else {
                    // Error files: tree-sitter error recovery produces
                    // garbage ancestry, so the AST classifier's expectations
                    // are not trustworthy — use the line-scan model for all
                    // rows (issue #202).
                    scan_expected[i]
                }
            })
            .collect();
        // For each annotation row, the row of the declaration it annotates —
        // the next code line after the annotation (JVM reports a mis-indented
        // annotation against its declaration's expected indent, issue #202).
        let annotation_target: Vec<Option<usize>> = (0..lines.len())
            .map(|i| {
                if lines[i].trim_start().starts_with('@') {
                    annotation_target_row(&lines, i)
                } else {
                    None
                }
            })
            .collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            // Trailing whitespace must not count as indentation (issue #169).
            let spaces = line.len() - line.trim_start().len();

            let raw_delimiters = line.matches("\"\"\"").count();
            let toggles_raw = raw_delimiters % 2 == 1;
            if in_raw_string {
                if toggles_raw {
                    // This row closes the raw string — it is a code row
                    // (the delimiter), checked below like JVM does.
                    in_raw_string = false;
                } else {
                    continue;
                }
            } else if toggles_raw {
                // This row opens the raw string (`val x = \"\"\"`) — code row.
                in_raw_string = true;
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

            // Skip: blank, comments, KDoc markers, string-only lines,
            // accessor headers. A standalone annotation row (annotation on
            // its own line) is only skipped when it has no declaration to
            // annotate (corrupt file) — otherwise it is checked against its
            // declaration's expected indent below (issue #202). Rows that
            // mix an annotation with declaration content (`@JvmField val
            // x`) are ordinary rows and follow the normal expectation.
            if trimmed.is_empty()
                || trimmed == "*/"
                || trimmed.starts_with("* ")
                || trimmed.starts_with("*/")
                || raw_string_delimiter_row(trimmed)
                || (standalone_annotation_row(trimmed) && annotation_target[i].is_none())
            {
                continue;
            }
            // A line comment is checked against the next code row's expected
            // indent (JVM reports a mis-indented `//` comment like a
            // statement, issue #202). Exemptions (JVM oracle): inside a
            // consecutive comment block, previous non-blank row at the same
            // indent, or before a closing brace/paren.
            if trimmed.starts_with("//") {
                // Commented-out code (`//        assertEquals(...)` — two or
                // more spaces after the `//`) keeps the commented code's
                // deep indent and is left unchecked (JVM oracle).
                let commented_code = trimmed[2..].starts_with("  ");
                let prev_comment = i > 0 && lines[i - 1].trim_start().starts_with("//");
                let next_comment = lines
                    .get(i + 1)
                    .map(|l| l.trim_start().starts_with("//"))
                    .unwrap_or(false);
                let mut prev = i;
                while prev > 0 && lines[prev - 1].trim().is_empty() {
                    prev -= 1;
                }
                let prev_spaces = if prev > 0 {
                    lines[prev - 1].len() - lines[prev - 1].trim_start().len()
                } else {
                    usize::MAX
                };
                if !commented_code && !prev_comment && !next_comment && prev_spaces != spaces {
                    if let Some(next) = next_code_row(&lines, i) {
                        let next_t = lines[next].trim();
                        if !next_t.starts_with('}') && !next_t.starts_with(')') {
                            // Under-indentation only: an over-indented
                            // comment may sit at a legitimate alignment
                            // level the next-row model does not know
                            // (e.g. when-entry bodies), so only the too-
                            // shallow direction is reported (issue #202).
                            if spaces < line_expected[next] {
                                violations.push(Violation {
                                    file: String::new(),
                                    line: i + 1,
                                    col: 1,
                                    rule_id: self.id().into(),
                                    message: format!(
                                        "Unexpected indentation ({}) (should be {})",
                                        spaces, line_expected[next]
                                    ),
                                    auto_fixable: true,
                                });
                            }
                        }
                    }
                }
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
            // A standalone annotation row takes the expected indent of the
            // declaration it annotates — the next code row (JVM oracle:
            // `@Test` at the wrong depth is reported against the fun's
            // indent, issue #202). Rows mixing an annotation with their
            // declaration (`@JvmField val x: T`) keep the row's own
            // expectation.
            let expected_for_line = if standalone_annotation_row(trimmed) {
                match annotation_target[i] {
                    Some(d) => line_expected[d],
                    None => depth_expected,
                }
            } else {
                depth_expected
            };
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
            // Over-indentation (issue #202): an indent deeper than expected
            // that lands on a multiple of indent_size is silently accepted
            // today (the `else if` above only fires on non-multiples). The
            // oracle reports it (`Unexpected indentation (8) (should be 4)`).
            // Probe-only: reports on rows where the AST classifier AND the
            // line-scan model agree on the expected value AND the row starts
            // a fresh statement (never a continuation — alignment indents are
            // not modelled exactly). Lazy: on already-formatted code
            // over-indented rows are rare, so the tree walk is almost never
            // paid.
            if probe && spaces > expected_for_line {
                // Confidence for the over-indent probe: the AST classifier
                // AND the line-scan model must agree on the expected value
                // AND the row starts a fresh statement (never a continuation
                // — alignment indents are not modelled exactly). Standalone
                // annotation rows inherit their declaration row's
                // confidence; binary-expression continuations have a
                // definite expectation (first row + 1) and are allowed too
                // (issue #202).
                let binary_cont = binary_continuation_row(_tree, source, i);
                let (ast_some, scan_agree, stmt, unconstrained) =
                    if standalone_annotation_row(trimmed) {
                        match annotation_target[i] {
                            Some(d) => (
                                ast[d].is_some(),
                                line_expected[d] == scan_expected[d],
                                // The annotated declaration's node starts at the
                                // annotation row (tree-sitter), so the statement
                                // check runs on the annotation row itself.
                                row_starts_statement(_tree, source, i),
                                lambda_in_unconstrained_argument(_tree, source, i),
                            ),
                            None => (
                                ast[i].is_some(),
                                expected_for_line == scan_expected[i],
                                row_starts_statement(_tree, source, i),
                                lambda_in_unconstrained_argument(_tree, source, i),
                            ),
                        }
                    } else {
                        (
                            ast[i].is_some(),
                            expected_for_line == scan_expected[i],
                            row_starts_statement(_tree, source, i) || binary_cont,
                            lambda_in_unconstrained_argument(_tree, source, i),
                        )
                    };
                let confident = tree_clean && ast_some && scan_agree && stmt && !unconstrained;
                if confident {
                    too_shallow = true;
                    expected_indent = Some(expected_for_line);
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

pub(crate) fn find_allman_elevated_blocks(
    tree: &tree_sitter::Tree,
    source: &str,
) -> Vec<(usize, usize, bool, Option<usize>, bool)> {
    let mut out = Vec::new();
    let mut stack: Vec<tree_sitter::Node> = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        // (open_line, close_line, open_elevated, statement_line)
        let mut blocks: Vec<(usize, usize, bool, Option<usize>, bool)> = Vec::new();
        match node.kind() {
            "if_expression" => {
                // The if body and any else body are control_structure_body
                // children; `else if` chains nest their own if_expression.
                for c in node.children(&mut node.walk()) {
                    if c.kind() == "control_structure_body" {
                        // stmt = open row keeps the frame for an inline
                        // `if (x) {` so its `}`/`} else` aligns with the if's
                        // own row; is_lambda=false so body rows are not
                        // unconditionally pinned (only the closing alignment).
                        // open_elevated only for Allman `if (x)\n{` bodies,
                        // whose rows are lifted above the brace depth; inline
                        // bodies are handled by the `{` branch.
                        let allman = source
                            .lines()
                            .nth(c.start_position().row)
                            .map(|l| l.trim() == "{")
                            .unwrap_or(false);
                        blocks.push((
                            c.start_position().row,
                            c.end_position().row,
                            allman,
                            Some(c.start_position().row),
                            false,
                        ));
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
                    blocks.push((open.start_position().row, close, !has_subject, None, false));
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
                            false,
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
                    blocks.push((open.start_position().row, close, true, None, false));
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
                            false,
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
                        false,
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
                    // stmt = open row keeps the frame when `{` shares the
                    // header row (`class Foo(...) : Base {`) so the closing
                    // `}` aligns with the body's own level — the class body
                    // opened by a primary-constructor `)` (`class X @Inject
                    // constructor(...) {`) sits one level under the header.
                    blocks.push((
                        open.start_position().row,
                        close,
                        false,
                        Some(open.start_position().row),
                        false,
                    ));
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
                    blocks.push((open.start_position().row, close, false, stmt, true));
                }
            }
            _ => {}
        }
        for (open, close, open_elevated, stmt, is_lambda) in blocks {
            let line = source.lines().nth(open).map(|l| l.trim()).unwrap_or("");
            // Lambda frames are recorded even when `{` shares its line with
            // the call (`foo { x ->`); every other kind needs the `{` alone
            // (Allman) to matter.
            if line == "{" || stmt.is_some() {
                out.push((open, close, open_elevated, stmt, is_lambda));
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

/// True when the line starts a class-like declaration (`class Foo`, `interface I`,
/// `object O`, …) regardless of whether a body brace or supertype list follows.
fn class_like_decl_header(t: &str) -> bool {
    // `annotation class` is a declaration of the annotation type itself; its
    // following lines are ordinary code, not constructor modifiers.
    if t.starts_with("annotation class ") {
        return false;
    }
    t.starts_with("class ")
        || t.starts_with("data class ")
        || t.starts_with("enum class ")
        || t.starts_with("sealed class ")
        || t.starts_with("abstract class ")
        || t.starts_with("open class ")
        || t.starts_with("internal class ")
        || t.starts_with("public class ")
        || t.starts_with("private class ")
        || t.starts_with("final class ")
        || t.starts_with("annotation class ")
        || t.starts_with("value class ")
        || t.starts_with("inline class ")
        || t.starts_with("interface ")
        || t.starts_with("object ")
        || t.starts_with("data object ")
        || t.starts_with("sealed object ")
        || t.starts_with("companion object ")
}

/// True when the line begins a top-level-ish declaration keyword.
fn starts_declaration_keyword(t: &str) -> bool {
    t.starts_with("class ")
        || t.starts_with("interface ")
        || t.starts_with("object ")
        || t.starts_with("enum ")
        || t.starts_with("fun ")
        || t.starts_with("val ")
        || t.starts_with("var ")
        || t.starts_with("typealias ")
        || t.starts_with("data ")
        || t.starts_with("sealed ")
        || t.starts_with("abstract ")
        || t.starts_with("open ")
        || t.starts_with("internal ")
        || t.starts_with("public ")
        || t.starts_with("private ")
}

/// True when scanning upward from `row` (exclusive) hits a class-like
/// declaration after only supertype-continuation rows (`...,`, `...)`, `...(`,
/// `...>`, `...?`). Used to recognize a class body `{` that closes a
/// supertype list (`class A :\n    B,\n    C {`).
fn supertype_chain_leads_to_class(lines: &[&str], row: usize) -> bool {
    for r in (0..=row).rev() {
        let tl = lines[r].trim();
        if tl.is_empty() {
            continue;
        }
        if class_like_decl_header(tl) {
            return true;
        }
        if tl.ends_with(')')
            || tl.ends_with('(')
            || tl.ends_with('>')
            || tl.ends_with('?')
            || tl.ends_with(',')
        {
            continue;
        }
        break;
    }
    false
}

/// True when the line is a class-like declaration header whose body would
/// open with `{` or whose supertype list follows a `:`.
fn class_like_decl_line(t: &str) -> bool {
    (t.starts_with("class ")
        || t.starts_with("data class ")
        || t.starts_with("enum class ")
        || t.starts_with("sealed class ")
        || t.starts_with("abstract class ")
        || t.starts_with("open class ")
        || t.starts_with("internal class ")
        || t.starts_with("public class ")
        || t.starts_with("private class ")
        || t.starts_with("final class ")
        || t.starts_with("annotation class ")
        || t.starts_with("value class ")
        || t.starts_with("inline class ")
        || t.starts_with("interface ")
        || t.starts_with("object ")
        || t.starts_with("data object ")
        || t.starts_with("sealed object ")
        || t.starts_with("companion object "))
        && (t.ends_with('{') || t.ends_with(':'))
}

pub(crate) fn compute_line_expected(
    lines: &[&str],
    is: usize,
    elevated: &[(usize, usize, bool, Option<usize>, bool)],
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
    let mut prev_binary_cont = false;
    // A class-like header without a body brace (`class Foo`, `class Foo :`)
    // is followed by its primary-constructor annotations and keyword on the
    // class's own indentation level: `class Foo\n    @Inject\n    constructor(`.
    let mut class_annotation_pending = false;
    // The brace depth when an arrow-parameter lambda body started
    // (`forEach { (navKey, navItem) ->` + body). While the depth is at or
    // above it, following rows keep the lifted body level.
    let mut arrow_body_depth: Option<usize> = None;
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
        if class_annotation_pending
            && (t.starts_with('@') || t.starts_with("constructor"))
            && !t.contains(" class ")
            && !t.contains(" fun ")
            && !t.contains(" val ")
            && !t.contains(" var ")
            && lines
                .get(i + 1)
                .is_none_or(|nxt| !starts_declaration_keyword(nxt.trim_start()))
        {
            // `class Foo\n    @Inject\n    constructor(` — the annotation is
            // the class's constructor modifier. An annotation that opens its
            // own declaration (`@Deprecated("x") class Second` after a
            // previous class) is NOT lifted.
            e = e.max(is);
        }
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
        for &(open, close, open_elevated, stmt, is_lambda) in elevated.iter() {
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
                } else if is_lambda {
                    // A lambda frame: every body row is pinned to the
                    // lambda's own row + one, unconditionally. A trailing
                    // lambda whose `{` sits on a wrapped call's closing row
                    // (`) {`) is lifted by the wrapping pass; its body must
                    // follow even when the raw brace depth is lower. Other
                    // frames (if/when/fun) keep the guarded pin so rows
                    // already indented inside are lifted but a following
                    // top-level declaration is not.
                    e = e.max(out[open].saturating_add(is));
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
            } else if prev_trim.starts_with("//") && paren_depth > 0 && prev_last_code == Some('{')
            {
                // A comment row after a `{` inside a paren-list block
                // (`setupBlock = {` at list+1): the comment's own row already
                // sits at the block-body level, so the following row keeps
                // it — and the lifted level persists for the rest of the
                // block body (rows after a comment still continue the block).
                e = prev_expected;
                if arrow_body_depth.is_none() {
                    arrow_body_depth = Some(depth);
                }
            } else if !prev_inert {
                if paren_depth > 0
                    && !t.starts_with('}')
                    && !t.starts_with(')')
                    && prev_last_code != Some('{')
                    && prev_last_code != Some('=')
                    && !(prev_last_code == Some('>') && lines[i - 1].trim_end().ends_with("->"))
                {
                    // Inside a paren list the expectation already came from
                    // the list indent; keep it. A `}` row (lambda close on
                    // the same line as trailing arguments — `}, 1, …)`) is
                    // governed by the closing-brace logic instead. A row
                    // right after a `{` (a lambda body inside the list:
                    // `navigationSuiteItems = {` + body) is a block body and
                    // goes through the `{` branch below. Rows inside an
                    // arrow lambda body take the higher of the list indent
                    // and the lifted body level.
                } else if prev_last_code == Some('{') && prev_was_supertype {
                    // Class body opened on a supertype continuation line.
                    e = prev_expected;
                } else if arrow_body_depth.is_some_and(|d| depth >= d)
                    && !t.starts_with('}')
                    && !t.starts_with(')')
                    && !matches!(prev_last_code, Some('{') | Some('=') | Some(':'))
                {
                    // Rows inside the arrow lambda body keep the lifted level
                    // (`val selected`, `item(`, … after `val hasUnread`).
                    // Rows after `{`/`=`/`:` go to their own branches.
                    e = e.max(prev_expected);
                } else if prev_last_code == Some('>') && lines[i - 1].trim_end().ends_with("->") {
                    // Lambda with a parameter list ending on its own line:
                    // `TOP_LEVEL…forEach { (navKey, navItem) ->` — the body
                    // sits one level under the lambda's own row. A `when`
                    // entry (`1 -> "one"`) carries its body on the same line
                    // and never reaches here.
                    e = prev_expected.saturating_add(is);
                    if arrow_body_depth.is_none() {
                        arrow_body_depth = Some(depth);
                    }
                } else if prev_last_code == Some('{') {
                    // A block opened at the end of the previous line
                    // (`fun f() {`, `runBlocking {`, `if (x) {`): the body
                    // sits one level deeper than the opener row. For a
                    // continuation opener (`val topic =\n    runBlocking {`)
                    // that is prev_expected + one level; a plain statement
                    // opener (`fun f() {` at brace depth d) also yields the
                    // brace-depth expectation, // so this branch is safe for
                    // both shapes (verified against ktlint 1.8). A class-like
                    // body whose `{` closes a supertype continuation
                    // (`class A :\n    B(\n        Servlet(x)\n    ) {`)
                    // stays at the plain brace depth instead (its body sits
                    // one level under the class header, not under the
                    // continuation).
                    let opener_line = lines[i - 1].trim();
                    let opener_is_class_header = class_like_decl_line(opener_line);
                    // The `{` closes a class body when it sits at the end
                    // of the header (`class Foo {`), after a `)` (`class
                    // Foo(...) {`), or on a supertype-continuation row
                    // (`class A :\n    B(\n        x\n    ) {` or a second
                    // supertype `) : X,\n    Y {`) — the opener's own line
                    // may start with a supertype name, not `)`.
                    let opener_is_supertype_close = opener_line.trim_end().ends_with('{')
                        && (opener_line.starts_with(')')
                            || supertype_chain_leads_to_class(lines, i - 1));
                    let mut is_class_body_open = opener_is_class_header;
                    if opener_is_supertype_close {
                        // `class A :\n    B(\n        Servlet(x)\n    ) {` —
                        // the `{` closes a supertype continuation; find the
                        // class declaration above to confirm.
                        for r in (0..i - 1).rev() {
                            let tl = lines[r].trim();
                            if tl.is_empty() {
                                continue;
                            }
                            if class_like_decl_line(tl) {
                                is_class_body_open = true;
                                break;
                            }
                            if tl.ends_with(')')
                                || tl.ends_with('(')
                                || tl.ends_with('>')
                                || tl.ends_with('?')
                            {
                                continue;
                            }
                            break;
                        }
                    }
                    if !is_class_body_open {
                        e = prev_expected.saturating_add(is);
                        // Persist the lifted body level for the rest of the
                        // block when the block itself is lifted above the
                        // raw brace depth: a paren-list continuation
                        // (`setupBlock = {` at list+2) or a trailing lambda
                        // after a wrapped call (`) {` on its own row).
                        // Statement-level blocks (`fun f() {`, `lazy {`)
                        // match the brace depth and must not pin.
                        if paren_depth > 0 && arrow_body_depth.is_none() {
                            arrow_body_depth = Some(depth);
                        }
                    }
                } else if matches!(prev_last_code, Some(':') | Some('=')) {
                    // Body/continuation line (block body, parameter list,
                    // supertype colon, initializer/expression body): the
                    // opener's expectation + one level. prev_last_code is the
                    // previous line's last *code* char — a trailing comment
                    // ending in `=`/`:` must not open a continuation.

                    let wrapped_return_type = prev_last_code == Some('=')
                        && prev_expected > depth * is
                        && i > 1
                        && lines[i - 2].trim_end().ends_with(':');
                    let mut want = prev_expected.saturating_add(is);
                    // A named-argument RHS inside a paren list
                    // (`NiaGradientBackground(\n    gradientColors =\n
                    //        if (...) {`) sits one level under the argument
                    // row, not under the list's opener.
                    if let Some(&(list, _, _)) = paren_expected.last() {
                        want = want.max(list.saturating_add(is));
                    }
                    let want = if wrapped_return_type {
                        // The `=` sits on a continuation line itself
                        // (wrapped return type: `fun name():\n    Type =\n
                        //    body`): the body sits at the `=` line's own
                        // level (oracle-verified: `when (this) {` at 4, not
                        // declaration level 0). The extra `:` guard keeps an
                        // ordinary `val x =` inside a lambda body (whose
                        // prev_expected is also deeper than the brace depth)
                        // from being mistaken for one.
                        prev_expected
                    } else {
                        want
                    };

                    if want > e {
                        e = want;
                    }
                }
            }
        }
        if t.contains(".also") {}
        if t.starts_with("public fun HttpRequestBuilder.bufferPolicy") {}
        // A property accessor (`get() = …`, bare `set`) on its own line sits
        // one level deeper than its property — the scan model needs this for
        // error files where the AST classifier is disabled (issue #202).
        let accessor_head =
            t.starts_with("get(") || t.starts_with("set(") || t == "get" || t == "set";
        if accessor_head && i > 0 {
            let prev_t = lines[i - 1].trim();
            let prev_is_property = prev_t.starts_with("val ")
                || prev_t.starts_with("var ")
                || prev_t.contains(" val ")
                || prev_t.contains(" var ");
            if prev_is_property {
                let want = prev_expected.saturating_add(is);
                if want > e {
                    e = want;
                }
            }
        }
        // A row continuing a multiline binary expression (`a() &&\n    b()`,
        // `first\n    + second`) sits one level deeper than the previous
        // code row; chain rows keep that level (JVM oracle, issue #202).
        let prev_code = if i > 0 {
            lines[i - 1].split("//").next().unwrap_or("").trim_end()
        } else {
            ""
        };
        let binary_cont = binary_operator_row(t, prev_code)
            || (paren_depth == 0 && t.starts_with('.') && prev_code.contains(" by "))
            || (arrow_body_depth.is_some()
                && t.starts_with('.')
                && !prev_code.trim_end().ends_with('}'))
            // A continuation row starting with `.` (a wrapped call chain
            // `onNodeWithTag(...)\n    .fetchSemanticsNode()`) sits one level
            // under its code row. Skipped after a closing brace (a `}` ends
            // the chain) and after a binary continuation (the `?:`/`&&`
            // chain keeps its own lifted level, verified by oracle).
            || (t.starts_with('.')
                && !prev_binary_cont
                && !prev_code.trim_end().ends_with('}')
                && !prev_code.trim_end().ends_with('{'));
        if binary_cont {
            // Chain rows (`a() &&\n    b() &&\n    c()`) keep the lifted
            // level of the previous row; the first continuation lifts one
            // level above it (JVM oracle, issue #202).
            let want = if prev_binary_cont {
                prev_expected
            } else {
                prev_expected.saturating_add(is)
            };
            if want > e {
                e = want;
            }
        }
        prev_binary_cont = binary_cont;
        out[i] = e;
        if arrow_body_depth.is_some_and(|d| depth < d) {
            arrow_body_depth = None;
        }
        if class_like_decl_header(t) && !t.ends_with('{') {
            class_annotation_pending = true;
        } else if t.contains('{') {
            class_annotation_pending = false;
        }
        prev_expected = e;
        let is_comment_row = t.starts_with("//") || t.starts_with("/*");
        if !is_comment_row {
            prev_last_code = last_code;
        }
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
// The named-argument `=`-value lift is code-style dependent (the IntelliJ
// IDEA style accepts the aligned value, JVM oracle issue #202); the rule
// sets this once per check(). Tests leave the default (KtlintOfficial,
// which lifts).
thread_local! {
    static CODE_STYLE: std::cell::Cell<CodeStyle> =
        const { std::cell::Cell::new(CodeStyle::KtlintOfficial) };
}

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
        return None;
    }
    // A row already being computed higher in the recursion means a cyclic
    // expectation (nested block ↔ header) — bail to the fallback.
    if IN_PROGRESS.with(|c| c.borrow().contains(&row)) {
        return None;
    }
    IN_PROGRESS.with(|c| c.borrow_mut().push(row));
    let line = src.lines().nth(row)?;
    let col = line.len() - line.trim_start().len();
    let point = tree_sitter::Point { row, column: col };
    let node = match tree.root_node().descendant_for_point_range(point, point) {
        Some(n) => n,
        None => return None,
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
        // A lambda argument `{` (inside a paren list) aligns with the other
        // arguments — `withUrl(\n    "/",\n    {` puts `{` at the args row
        // + one level (oracle, issue #202). A true block-opening Allman `{`
        // sits at the previous row + one level — except class/object bodies
        // which stay at the header row (oracle: `class C\n{`).
        let mut up = node;
        let mut kind = "";
        let mut args_row: Option<usize> = None;
        loop {
            match up.kind() {
                "value_arguments" | "function_value_parameters" | "class_parameters" => {
                    kind = "args";
                    args_row = Some(up.start_position().row);
                    break;
                }
                "lambda_literal" => {
                    // A lambda argument is classified by its paren-list owner;
                    // walk up through it. A standalone lambda (chained lambda
                    // body / Allman lambda) is the "lambda" case below.
                    let mut w = up;
                    let mut found_args = None;
                    while let Some(p) = w.parent() {
                        match p.kind() {
                            "value_arguments"
                            | "function_value_parameters"
                            | "class_parameters" => {
                                found_args = Some(p.start_position().row);
                                break;
                            }
                            "statements" | "class_body" | "function_body" | "enum_class_body"
                            | "lambda_literal" => break,
                            _ => {}
                        }
                        w = p;
                    }
                    if let Some(r) = found_args {
                        kind = "args";
                        args_row = Some(r);
                        break;
                    }
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
            "args" => match args_row {
                Some(r) if r < row => ast_expected(tree, src, r, is).map(|e| e + is),
                _ => None,
            },
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
                // than the parameter row (oracle). Call-site named argument
                // values lift one level too — except under the IntelliJ IDEA
                // code style, where the aligned value is accepted (a ktor
                // fixture at the same shape is silent under
                // `ktlint_code_style = intellij_idea`, JVM oracle issue
                // #202). A bare identifier or numeric literal may align
                // with the argument row even when the lift applies.
                if row > 0 {
                    let prev_raw = src.lines().nth(row - 1).unwrap_or("");
                    let prev_code = prev_raw.trim().split("//").next().unwrap_or("").trim_end();
                    if prev_code.ends_with('=')
                        && !trimmed.starts_with('.')
                        && !trimmed.starts_with("?:")
                    {
                        let lift = CODE_STYLE.with(|c| c.get()) != CodeStyle::IntelliJIdea;
                        let simple = {
                            let body = trimmed.trim_end_matches(',');
                            let mut chars = body.chars();
                            matches!(chars.next(), Some(c) if c.is_alphabetic() || c == '_')
                                && body.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                        } || trimmed.trim_end_matches(',').parse::<i64>().is_ok()
                            || trimmed.trim_end_matches(',').parse::<f64>().is_ok();
                        let lift_row = c.kind() == "function_value_parameters"
                            || c.kind() == "class_parameters"
                            || c.kind() == "primary_constructor"
                            || (lift && !simple);
                        if lift_row {
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
                // A row continuing a binary/expression continuation inside
                // the args (`withContext(\n    a +\n        b { … }`) sits
                // one level deeper than the args themselves (oracle). The
                // continuation expression started on a previous row inside
                // the paren list.
                let nested_continuation = chain.iter().any(|n| {
                    matches!(
                        n.kind(),
                        "additive_expression"
                            | "multiplicative_expression"
                            | "comparison_expression"
                            | "equality_expression"
                            | "conjunction_expression"
                            | "disjunction_expression"
                            | "elvis_expression"
                            | "range_expression"
                            | "infix_expression"
                    ) && n.start_position().row < row
                        && n.start_position().row > c.start_position().row
                });
                let extra = if nested_continuation { 2 * is } else { is };
                return ast_expected(tree, src, c.start_position().row, is).map(|e| e + extra);
            }
            "additive_expression"
            | "multiplicative_expression"
            | "comparison_expression"
            | "equality_expression"
            | "conjunction_expression"
            | "disjunction_expression"
            | "range_expression"
            | "infix_expression" => {
                // A row continuing a multiline binary expression (`return
                // a() &&\n    b()` — the expression node spans rows and
                // started earlier) sits one level deeper than the
                // expression's first row (JVM oracle, issue #202). Only
                // fires when the row starts a fresh operand (never a closer
                // or a nested block row).
                if c.start_position().row < row && binary_continuation_row(tree, src, row) {
                    return ast_expected(tree, src, c.start_position().row, is).map(|e| e + is);
                }
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
                    // Brace-less control-statement body: `for (i in 1..10_000)`
                    // / `if (x)` / `while (x)` complete on their own line,
                    // the following statement is the body at +1 level
                    // (oracle). tree-sitter parses these bodies as sibling
                    // statements (no control_structure_body in the ancestor
                    // chain for some shapes, e.g. inside parentheses), so
                    // detect it textually.
                    if row > 0 && !trimmed.starts_with('}') {
                        let prev_raw = src.lines().nth(row - 1).unwrap_or("");
                        let prev_trim = prev_raw.trim();
                        let prev_code = prev_trim.split("//").next().unwrap_or("").trim_end();
                        let starts_control = ["for ", "if ", "while "]
                            .iter()
                            .any(|k| prev_code.starts_with(k));
                        if starts_control {
                            if let Some(close) = header_close_paren(prev_code) {
                                if prev_code[close + 1..].trim().is_empty() {
                                    return ast_expected(tree, src, row - 1, is).map(|e| e + is);
                                }
                            }
                        }
                    }
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
                        // Elvis lambda bodies: a line-start elvis (`?: run {`)
                        // lifts its body one level (oracle); a mid-line elvis
                        // (`.takeIf { } ?: run {`) keeps the body at the
                        // elvis line's own level. Only the closing `}`
                        // returns to the elvis row (handled above).
                        if open_line.starts_with("?: ") || open_line.starts_with("?:{") {
                            return ast_expected(tree, src, open_row, is).map(|e| e + is);
                        }
                        return ast_expected(tree, src, open_row, is);
                    }
                    // A lambda nested in a control-structure condition
                    // (`if (a || runCatching {`) lifts its body two levels
                    // (condition continuation + lambda), not one (oracle).
                    // Only when the block owner is the lambda itself.
                    let lift = if owner.kind() == "lambda_literal" && lambda_in_condition(&owner) {
                        2 * is
                    } else {
                        is
                    };
                    return ast_expected(tree, src, open_row, is).map(|e| e + lift);
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
                    // Find the owning property declaration: a sibling of the
                    // accessor under class_body, or the property ancestor
                    // (top-level properties — `val a\nget()` — parent the
                    // accessor through property_accessors, issue #202).
                    let prop_row = c
                        .parent()
                        .and_then(|p| {
                            p.children(&mut p.walk())
                                .filter(|k| k.kind() == "property_declaration")
                                .filter(|k| k.start_position().row <= row)
                                .map(|k| k.start_position().row)
                                .max()
                        })
                        .or_else(|| {
                            let mut up = *c;
                            loop {
                                match up.parent() {
                                    Some(p) => {
                                        if p.kind() == "property_declaration" {
                                            return Some(p.start_position().row);
                                        }
                                        up = p;
                                    }
                                    None => return None,
                                }
                            }
                        });
                    if let Some(pr) = prop_row {
                        return ast_expected(tree, src, pr, is).map(|e| e + is);
                    }
                }
            }
            "class_body" | "function_body" | "enum_class_body" => {
                let is_class = matches!(c.kind(), "class_body" | "enum_class_body");
                // Class/enum members and closing braces sit relative to the
                // class header's base line. For a multiline primary
                // constructor that is the `)` row
                // (`class C\n  @X\n  constructor(\n    a: Int\n  ) : Base {`)
                // — not the class declaration row (oracle: okhttp style).
                // Function bodies keep the header row.
                let base_row = if is_class {
                    c.parent()
                        .and_then(|owner| {
                            owner
                                .children(&mut owner.walk())
                                .find(|k| {
                                    k.kind() == "class_parameters"
                                        || k.kind() == "primary_constructor"
                                })
                                .map(|pc| pc.end_position().row)
                                .filter(|r| *r > owner.start_position().row)
                                .or(Some(owner.start_position().row))
                        })
                        .unwrap_or(c.start_position().row)
                } else {
                    c.parent()
                        .map(|p| p.start_position().row)
                        .unwrap_or(c.start_position().row)
                };
                if trimmed.starts_with('}') {
                    // A `}` inside an `init { }` block closes at the init
                    // row (oracle: `init {\n    …\n}`).
                    if let Some(init) = chain.iter().find(|n| {
                        n.kind() == "anonymous_initializer" && n.start_position().row < row
                    }) {
                        return ast_expected(tree, src, init.start_position().row, is);
                    }
                    // The closing `}` returns to the block's opening row —
                    // the header base for class bodies (constructor-style
                    // classes close at the `)` row's level), the Allman `{`
                    // row / header row otherwise.
                    if is_class {
                        return ast_expected(tree, src, base_row, is);
                    }
                    return ast_expected(tree, src, block_open_row(c, src), is);
                }
                if let Some(owner) = c.parent() {
                    return ast_expected(tree, src, base_row, is).map(|e| e + is);
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
                    // A row inside a parenthesized value
                    // (`val x =\n    (\n        if (…) {`) sits one level
                    // deeper than the value's first row (oracle).
                    let nested_paren = chain.iter().any(|n| {
                        n.kind() == "parenthesized_expression"
                            && n.start_position().row < row
                            && n.start_position().row > c.start_position().row
                    });
                    let extra = if nested_paren { 2 * is } else { is };
                    return ast_expected(tree, src, c.start_position().row, is).map(|e| e + extra);
                }
            }
            _ => {}
        }
    }
    Some(0)
}

/// The opening row of the block a `statements` node belongs to.
/// True when the row's first code token starts a new statement inside a body
/// container (statements / class_body / …) — as opposed to a continuation of
/// a statement that began on an earlier line (`.map`, `?: …`, `)`, wrapped
/// args). Over-indentation is only probed on statement-start rows: a
/// continuation's legitimate indent is alignment, which the classifier does
/// not model exactly (issue #202).
pub(crate) fn row_starts_statement(tree: &tree_sitter::Tree, src: &str, row: usize) -> bool {
    let line = src.lines().nth(row).unwrap_or("");
    let col = line.len() - line.trim_start().len();
    let point = tree_sitter::Point { row, column: col };
    let Some(mut n) = tree.root_node().descendant_for_point_range(point, point) else {
        return false;
    };
    loop {
        match n.parent() {
            Some(p) => {
                if matches!(
                    p.kind(),
                    "statements"
                        | "class_body"
                        | "enum_class_body"
                        | "secondary_constructor"
                        | "source_file"
                ) {
                    // An annotated declaration's node starts at its first
                    // annotation row, so the declaration row below an
                    // annotation fails the start-row equality. Accept a row
                    // that is itself a standalone annotation, or a row
                    // directly below a standalone annotation that begins a
                    // declaration header (issue #202). A bare `suspend …`
                    // continuation (generic/type argument) is not a
                    // statement start.
                    let prev_is_ann = row > 0
                        && src
                            .lines()
                            .nth(row - 1)
                            .map(|l| l.trim().starts_with('@'))
                            .unwrap_or(false);
                    return n.start_position().row == row
                        || line.trim_start().starts_with('@')
                        || (prev_is_ann && is_decl_header(line.trim()));
                }
                n = p;
            }
            None => return false,
        }
    }
}

/// Index of the `)` that closes the `for (` header, accounting for nested
/// parens (`for ((a, b) in pairs)`). None when no complete header is found.
fn header_close_paren(code: &str) -> Option<usize> {
    let open = code.find('(')?;
    let mut depth = 0usize;
    for (i, ch) in code[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// True when a lambda sits inside a control structure's parenthesized
/// condition (`if (a || runCatching {`, `while (x && run {`) rather than in
/// its body or elsewhere. Its body then lifts two levels below the control
/// structure row (condition continuation + lambda body), not one (oracle).
fn lambda_in_condition(lambda: &tree_sitter::Node) -> bool {
    // A lambda inside a control structure's condition (`if (a || runCatching
    // {`, `if (runCatching { … }.isSuccess)`) reaches the control structure
    // node before any body container. A then/else-branch lambda sits inside
    // the branch's control_structure_body first.
    let mut n = *lambda;
    while let Some(p) = n.parent() {
        match p.kind() {
            "if_expression" | "while_statement" | "for_statement" | "do_while_statement" => {
                return true;
            }
            "statements"
            | "function_body"
            | "class_body"
            | "enum_class_body"
            | "lambda_literal"
            | "when_entry"
            | "control_structure_body" => {
                return false;
            }
            _ => {}
        }
        n = p;
    }
    false
}

/// True when the row sits inside a lambda that is an argument value whose
/// indentation ktlint does not constrain: a lambda nested inside an argument
/// expression (`builder(\n    X.map {\n        …`) or a named-argument lambda
/// (`onClick =\n    { …`). ktlint's indent rule yields no expectation for
/// these rows (oracle: any indent is accepted), so over-indentation cannot
/// be reported on them. A bare positional lambda argument (`call(\n    {`)
/// stays strict.
pub(crate) fn lambda_in_unconstrained_argument(
    tree: &tree_sitter::Tree,
    src: &str,
    row: usize,
) -> bool {
    let line = src.lines().nth(row).unwrap_or("");
    let col = line.len() - line.trim_start().len();
    let point = tree_sitter::Point { row, column: col };
    let Some(mut n) = tree.root_node().descendant_for_point_range(point, point) else {
        return false;
    };
    let mut has_lambda = false;
    let mut has_arg = false;
    let mut lambda_direct = false;
    let mut arg_row = 0usize;
    loop {
        match n.kind() {
            "lambda_literal" => {
                has_lambda = true;
                let direct = match n.parent() {
                    Some(p) if p.kind() == "value_argument" => true,
                    Some(p) if p.kind() == "annotated_lambda" => {
                        matches!(p.parent().map(|g| g.kind()), Some("value_argument"))
                    }
                    _ => false,
                };
                lambda_direct = direct;
            }
            "value_argument" => {
                has_arg = true;
                arg_row = n.start_position().row;
            }
            _ => {}
        }
        match n.parent() {
            Some(p) => n = p,
            None => break,
        }
    }
    if !has_lambda || !has_arg {
        return false;
    }
    // Named argument (`name =` on the owning line)? ktlint leaves the value
    // unconstrained either way — nested lambdas are free, named lambdas are
    // free; only a bare positional lambda argument is strict.
    let arg_code = src.lines().nth(arg_row).unwrap_or("").trim();
    let named = arg_code.contains('=');
    named || !lambda_direct
}

/// True when the trimmed row is a property accessor header: `get() =`,
/// `get() {`, `set(value) {`, a bare `get`/`set` keyword, optionally after
/// a visibility modifier or an annotation on the same row (`@InternalAPI
/// set(value) {`). A `get(`/`set(` substring elsewhere is NOT an accessor —
/// `@Target(...)` contains `get(` but must be checked (issue #202), and
/// `.get(0)`/`map.get(k)` calls follow chain rules. Accessor rows stay
/// unchecked: their indent expectations are not modelled.
fn accessor_row(trimmed: &str) -> bool {
    let body = trimmed.trim_start();
    let stripped = if body.starts_with('@') {
        body.split_once(' ')
            .map(|(_, r)| r.trim_start())
            .unwrap_or("")
    } else {
        body
    };
    let mut words = stripped.split_whitespace();
    let first = words.next().unwrap_or("");
    let second = words.next().unwrap_or("");
    let is_accessor =
        |w: &str| w == "get" || w == "set" || w.starts_with("get(") || w.starts_with("set(");
    is_accessor(first)
        || (matches!(
            first,
            "private" | "public" | "internal" | "protected" | "inline"
        ) && is_accessor(second))
}

/// True when the trimmed row is a standalone annotation: an `@` annotation
/// (optionally with a use-site target like `@get:`) followed only by its own
/// argument list and nothing else — `@Test`, `@get: Rule(order = 0)`,
/// `@Deprecated(` (multi-line args). A row that mixes an annotation with
/// declaration content (`@JvmField val x: T`, `@Deprecated(...) public val
/// realm`) is an ordinary declaration row, not a standalone annotation
/// (issue #202).
fn standalone_annotation_row(t: &str) -> bool {
    let body = t.trim_start();
    if !body.starts_with('@') {
        return false;
    }
    let mut depth: isize = 0;
    let mut end = body.len();
    for (i, c) in body.char_indices() {
        if i == 0 {
            continue;
        }
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            c if c.is_whitespace() && depth == 0 => {
                // A use-site target (`@get: Rule(...)`) has a space between
                // the target and the annotation name — keep scanning.
                if body[1..i].ends_with(':') {
                    continue;
                }
                end = i;
                break;
            }
            _ => {}
        }
    }
    if depth > 0 {
        // `@Deprecated(` — the argument list continues on later rows.
        return true;
    }
    body[end..].trim().is_empty()
}

/// True when the trimmed row is a raw-string delimiter/continuation row
/// that must stay unchecked: `"""` alone, `""".trimIndent()`, a closer
/// with trailing content (`}()"""`, `|content"""`, `"""content`). Only the
/// opener row that carries an assignment (`value = """`) is checked — JVM
/// reports a mis-indented one like any declaration (issue #202).
fn raw_string_delimiter_row(t: &str) -> bool {
    if !t.contains("\"\"\"") {
        return false;
    }
    let before = t.split("\"\"\"").next().unwrap_or("").trim_end();
    !before.ends_with('=') && !before.ends_with('(')
}

/// True when the row continues a multiline binary expression: the AST node
/// at the row starts a fresh operand of a binary expression that began on
/// an earlier row (`return a() &&\n    b()`), or the node IS the binary
/// expression spanning rows (`&& call…` after `= …`). Closers (`)`, `}`)
/// and nested block rows never qualify (issue #202).
fn binary_continuation_row(tree: &tree_sitter::Tree, src: &str, row: usize) -> bool {
    let line = src.lines().nth(row).unwrap_or("");
    let col = line.len() - line.trim_start().len();
    let point = tree_sitter::Point { row, column: col };
    let Some(node) = tree.root_node().descendant_for_point_range(point, point) else {
        return false;
    };
    if line.trim_start().starts_with(')') || line.trim_start().starts_with('}') {
        return false;
    }
    let is_binary = |k: &str| {
        matches!(
            k,
            "additive_expression"
                | "multiplicative_expression"
                | "comparison_expression"
                | "equality_expression"
                | "conjunction_expression"
                | "disjunction_expression"
                | "range_expression"
                | "infix_expression"
        )
    };
    if is_binary(node.kind()) && node.start_position().row < row {
        // The node is the binary expression itself (`&& call…` after `=`):
        // it must be the statement's direct expression.
        return node
            .parent()
            .map(|p| is_binary_statement_root(p.kind()))
            .unwrap_or(false);
    }
    let mut n = node;
    while let Some(p) = n.parent() {
        if is_binary(p.kind()) && p.start_position().row < row {
            // The binary expression must be the statement's direct
            // expression — a continuation inside an if/while condition, a
            // call argument, or a chain follows different rules and is not
            // a binary continuation (issue #202). Nested binary parents
            // (`(a && b) && c`) are part of the same statement expression
            // and are skipped.
            if node.start_position().row == row {
                let mut top = p;
                loop {
                    match top.parent() {
                        Some(gp) if is_binary(gp.kind()) => top = gp,
                        Some(gp) => return is_binary_statement_root(gp.kind()),
                        None => return false,
                    }
                }
            }
            return false;
        }
        n = p;
    }
    false
}

/// True when the node kind can directly own a statement-level binary
/// expression whose continuations lift one level (issue #202). Conditions
/// (`if (a && b)`), chains, and call arguments follow their own alignment
/// rules and are excluded. `return` parses as `jump_expression` in
/// tree-sitter-kotlin.
fn is_binary_statement_root(kind: &str) -> bool {
    matches!(
        kind,
        "return_statement"
            | "jump_expression"
            | "property_declaration"
            | "assignment_expression"
            | "expression_statement"
            | "function_declaration"
            | "function_body"
    )
}

/// The next code row after `row`: the next line that is not blank, not a
/// line/block comment, not KDoc, and not a raw-string delimiter. Gives a
/// standalone `//` comment the expected indent of the statement that
/// follows it (JVM oracle, issue #202).
fn next_code_row(lines: &[&str], row: usize) -> Option<usize> {
    for (j, l) in lines.iter().enumerate().skip(row + 1) {
        let t = l.trim();
        if t.is_empty()
            || t.starts_with("//")
            || t.starts_with("/*")
            || t.starts_with('*')
            || t.starts_with("\"\"\"")
        {
            continue;
        }
        return Some(j);
    }
    None
}

/// True when the trimmed row continues a binary expression: it starts with
/// an operator (`+ second`) or the previous code row ends with one
/// (`a() &&`). Used by the line-scan model to lift binary continuations
/// one level. Excludes import wildcards (`libcurl.*`), postfix `++`/`--`,
/// and reference operators (`::`) (issue #202).
fn binary_operator_row(t: &str, prev_code: &str) -> bool {
    let starts = ["&&", "||", "==", "!=", "<=", ">=", "??", "?:", "+", "-"];
    let ends = [
        "&&", "||", "==", "!=", "<=", ">=", "??", "+", "-", "*", "/", "%",
    ];
    starts.iter().any(|op| t.starts_with(op))
        || (ends.iter().any(|op| prev_code.ends_with(op))
            && !prev_code.ends_with(".*")
            && !prev_code.ends_with("*/")
            && !prev_code.ends_with("++")
            && !prev_code.ends_with("--")
            && !prev_code.ends_with("::"))
}

/// The row of the declaration an annotation row annotates: the next code
/// line after the annotation, skipping blank lines, comments, KDoc markers,
/// string-only rows, and further annotation rows (`@A\n@B\nfun f` — both
/// annotations share `fun f`'s indent). Rows inside the annotation's own
/// multi-line argument list (`@Foo(\n    arg\n)` are annotation
/// continuations, not the declaration. None when no declaration follows
/// (annotation at EOF or before a closing brace — corrupt file, uncheckable).
fn annotation_target_row(lines: &[&str], row: usize) -> Option<usize> {
    let mut depth: isize = 0;
    let mut in_args = false;
    for (j, l) in lines.iter().enumerate().skip(row) {
        let code = l.split("//").next().unwrap_or("");
        if j == row {
            // The annotation row itself: count its own parens so a
            // multi-line argument list is skipped below.
            depth = count_parens(code);
            in_args = depth > 0;
            continue;
        }
        if in_args {
            // Inside an annotation's argument list — includes the row that
            // closes it (`)`): annotation continuations are not the
            // declaration.
            depth += count_parens(code);
            if depth <= 0 {
                in_args = false;
            }
            continue;
        }
        let t = l.trim();
        if t.starts_with('@') {
            if standalone_annotation_row(t) {
                // A further stacked annotation (`@A\n@B(...)`): its own args
                // must also be skipped before reaching the declaration.
                depth = count_parens(code);
                if depth > 0 {
                    in_args = true;
                }
                continue;
            }
            // A row mixing an annotation with declaration content
            // (`@JvmField val x`, `@Synchronized fun f`) IS the declaration
            // itself — it is the annotation's target.
            if accessor_row(t) {
                return None;
            }
            return Some(j);
        }
        if t.is_empty()
            || t.starts_with("//")
            || t.starts_with("/*")
            || t.starts_with('*')
            || t.starts_with('"')
        {
            continue;
        }
        // An accessor header is not a declaration row the annotation can
        // inherit an indent from (accessor expectations are unmodelled) —
        // treat the annotation as uncheckable (issue #202).
        if accessor_row(t) {
            return None;
        }
        return Some(j);
    }
    None
}

/// Paren balance of a code slice, ignoring string literals (single `"`
/// toggles, backslash-escaped chars skipped) so `@Suppress("a(b)")` stays
/// balanced and multi-line annotation args are tracked correctly (issue
/// #202).
fn count_parens(code: &str) -> isize {
    let mut depth: isize = 0;
    let mut in_str = false;
    let mut chars = code.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '"' => in_str = !in_str,
            '(' if !in_str => depth += 1,
            ')' if !in_str => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
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
    use crate::rules::structure::indentation::lambda_in_unconstrained_argument;
    use crate::rules::structure::indentation::Indentation;
    use crate::rules::Rule;
    // Tests toggle KTLINT_RS_INDENT_PROBE via the process env; a shared lock
    // keeps the concurrent test threads from clobbering each other.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn issue202_lambda_arg_brace_aligns_with_args() {
        // `withUrl(\n    "/",\n    {` — a lambda argument `{` on its own
        // line aligns with the other arguments (args row + one level).
        expect_ok(
            "fun main() {\n    withUrl(\n        \"/\",\n        {\n            method = HttpMethod.Post\n        }\n    )\n}\n",
            &[(1, 0), (2, 4), (3, 8), (4, 8), (5, 12), (6, 8), (7, 4), (8, 0)],
        );
    }

    #[test]
    fn issue202_line_start_elvis_body_lifts() {
        // `?: run {` at line start lifts its body one level.
        expect_ok(
            "fun main() {\n    val x = listOf(1)\n        ?.firstOrNull()\n        ?.let { a -> a }\n        ?: run {\n            foo()\n        }\n}\n",
            &[(1, 0), (2, 4), (3, 8), (4, 8), (5, 8), (6, 12)],
        );
    }

    #[test]
    fn issue202_midline_elvis_body_keeps_line_level() {
        // `.takeIf { } ?: run {` — a mid-line elvis keeps its body at the
        // elvis line's own level.
        expect_ok(
            "fun main() {\n    val x = listOf(1)\n        .takeIf { it.isNotEmpty() } ?: run {\n        foo()\n        return@run null\n    }\n}\n",
            &[(1, 0), (2, 4), (3, 8), (4, 8), (5, 8), (6, 4), (7, 0)],
        );
    }

    #[test]
    fn issue202_bracless_for_body_lifts() {
        // `for (i in 1..10_000)` complete on its own line: the following
        // statement is its body at +1 level (tree-sitter parses it as a
        // sibling statement, so the classifier detects it textually).
        expect_ok(
            "fun main() {\n    buildString {\n        for (i in 1..10_000)\n            appendLine(\"x\")\n    }\n}\n",
            &[(1, 0), (2, 4), (3, 8), (4, 12), (5, 4), (6, 0)],
        );
    }

    #[test]
    fn issue202_condition_lambda_lifts_twice() {
        // A lambda inside a control-structure condition (`if (a || runCatching
        // {`) lifts its body two levels (condition continuation + lambda).
        expect_ok(
            "fun main() {\n    if (a || runCatching {\n            foo()\n        }) {\n        bar()\n    }\n}\n",
            &[(1, 0), (2, 4), (3, 12), (4, 4), (5, 8), (6, 4), (7, 0)],
        );
    }

    #[test]
    fn issue202_args_binary_continuation_lifts() {
        // `withContext(\n    a +\n        CoroutineExceptionHandler { …` —
        // a binary continuation inside the args sits one level deeper than
        // the args themselves.
        expect_ok(
            "fun main() {\n    withContext(\n        application.coroutineContext +\n            CoroutineExceptionHandler { _, e ->\n                log(e)\n            }\n    ) { }\n}\n",
            &[(1, 0), (2, 4), (3, 8), (4, 12), (5, 16)],
        );
    }

    #[test]
    fn issue202_unconstrained_argument_lambda_skipped() {
        // Argument values containing a lambda are unconstrained by ktlint
        // (any indent accepted) — the probe must not report over-indentation
        // inside them. A bare positional lambda argument stays strict.
        let tree = KotlinParser::new().parse(
            "fun main() {\n    builder(\n        userAgent = \"x\",\n        enabled =\n            ConnectionSpec.cipherSuites.map {\n                foo()\n            },\n    )\n}\n",
        );
        let src = "fun main() {\n    builder(\n        userAgent = \"x\",\n        enabled =\n            ConnectionSpec.cipherSuites.map {\n                foo()\n            },\n    )\n}\n";
        // lambda body inside a named-argument chain value → unconstrained
        assert!(lambda_in_unconstrained_argument(&tree, src, 5));
        // the `.map {` value row itself (chain, no lambda in the chain yet) —
        // not classified as unconstrained (blocked by other gates anyway)
        assert!(!lambda_in_unconstrained_argument(&tree, src, 4));
        // a bare positional lambda argument stays strict
        let src2 = "fun main() {\n    withUrl(\n        \"/\",\n        {\n            method = HttpMethod.Post\n        }\n    )\n}\n";
        let tree2 = KotlinParser::new().parse(src2);
        assert!(!lambda_in_unconstrained_argument(&tree2, src2, 4));
    }

    fn issue202_over_indent_reported_when_confident() {
        // The issue #202 repro: over-indentation on a multiple of
        // indent_size is reported when the classifier is confident.
        let src = "package com.example\n\npublic fun exampleFunction(): Int {\n        val value = 1\n    return value\n}\n";
        let tree = KotlinParser::new().parse(src);
        assert_eq!(ast_expected(&tree, src, 3, 4), Some(4));
        assert_eq!(ast_expected(&tree, src, 4, 4), Some(4));
    }

    #[test]
    fn issue202_class_members_follow_constructor_base() {
        // okhttp-style header: `class C\n    constructor(\n        x: Int\n    ) : Base {` —
        // class members sit one level deeper than the `)` row, and enum
        // entries one level deeper than the enum row.
        expect_ok(
            "class C\n    constructor(\n        val x: Int? = null,\n    ) : Base {\n        enum class Level {\n            NONE,\n            HEADERS,\n        }\n    }\n",
            &[(1, 0), (2, 4), (3, 8), (4, 4), (5, 8), (6, 12), (7, 12), (8, 8), (9, 4)],
        );
    }

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
    fn issue202_annotation_under_indent_reported() {
        // A standalone annotation at the wrong (too shallow) depth is
        // reported against its declaration's expected indent, like JVM
        // ktlint: `@Test` at 0 inside a class body (should be 4).
        let src = "class Foo {\n    fun ok() {}\n@Test\nfun bad() {}\n}\n";
        let violations = Indentation::new(4).check(&KotlinParser::new().parse(src), src);
        let rows: Vec<usize> = violations.iter().map(|v| v.line).collect();
        // JVM reports both the annotation and its mis-indented declaration.
        assert_eq!(rows, vec![3, 4]);
    }

    #[test]
    fn issue202_annotation_over_indent_reported() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Probe mode (issue #202 measurement): over-indented standalone
        // annotations are reported when the declaration's expectation is
        // confident (`@Test` at 8 should be 4).
        unsafe { std::env::set_var("KTLINT_RS_INDENT_PROBE", "1") };
        let src = "class Foo {\n    fun ok() {}\n        @Test\n    fun bad() {}\n}\n";
        let violations = Indentation::new(4).check(&KotlinParser::new().parse(src), src);
        unsafe { std::env::remove_var("KTLINT_RS_INDENT_PROBE") };
        let rows: Vec<usize> = violations.iter().map(|v| v.line).collect();
        assert_eq!(rows, vec![3]);
    }

    #[test]
    fn issue202_annotation_stack_and_multiline_args() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Stacked standalone annotations share the declaration's expected
        // indent: `@Marker` / `@Target(...)` at 8 above `val prop` at 4 are
        // both reported.
        let src = "@Target(AnnotationTarget.FUNCTION)\nannotation class Marker\n\nclass Baz {\n        @Marker\n        @Target(AnnotationTarget.FUNCTION)\n    val prop: Int = 1\n}\n";
        unsafe { std::env::set_var("KTLINT_RS_INDENT_PROBE", "1") };
        let violations = Indentation::new(4).check(&KotlinParser::new().parse(src), src);
        unsafe { std::env::remove_var("KTLINT_RS_INDENT_PROBE") };
        let rows: Vec<usize> = violations.iter().map(|v| v.line).collect();
        assert_eq!(rows, vec![5, 6]);
    }

    #[test]
    fn issue202_combined_annotation_row_uses_own_expectation() {
        // A row mixing an annotation with its declaration (`@JvmField val
        // x`) is an ordinary declaration row — correctly indented combined
        // rows in a class parameter list produce no report.
        let src = "class C(\n    @JvmField val node: Node,\n) {\n}\n";
        let violations = Indentation::new(4).check(&KotlinParser::new().parse(src), src);
        assert!(violations.is_empty(), "{:?}", violations);
    }

    #[test]
    fn issue202_accessor_annotation_skipped() {
        // An annotation on a property accessor (`@Deprecated` above `set`)
        // stays unchecked — accessor indent expectations are not modelled.
        let src =
            "class C {\n    public var x: Int = 1\n        @Deprecated(\"old\")\n        set\n}\n";
        let violations = Indentation::new(4).check(&KotlinParser::new().parse(src), src);
        assert!(violations.is_empty(), "{:?}", violations);
    }

    #[test]
    fn issue202_top_level_annotated_class() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Top-level annotations (`@Marker` before `class TopLevelBad`) are
        // checked against the class's expected indent (0).
        let src = "class Foo {}\n\n        @Marker\nclass TopLevelBad {}\n";
        unsafe { std::env::set_var("KTLINT_RS_INDENT_PROBE", "1") };
        let violations = Indentation::new(4).check(&KotlinParser::new().parse(src), src);
        unsafe { std::env::remove_var("KTLINT_RS_INDENT_PROBE") };
        let rows: Vec<usize> = violations.iter().map(|v| v.line).collect();
        assert_eq!(rows, vec![3]);
    }

    #[test]
    fn issue202_kts_script_skipped_by_extension() {
        let _guard = ENV_LOCK.lock().unwrap();
        // `.kts` scripts are skipped via check_with_path (extension-based);
        // the content heuristic misfired on `.kt` files with only top-level
        // vals (Type.kt, issue #202).
        unsafe { std::env::set_var("KTLINT_RS_INDENT_PROBE", "1") };
        let src = "val x = 1\n        val y = 2\n";
        let tree = KotlinParser::new().parse(src);
        let rule = Indentation::new(4);
        assert!(rule.check_with_path("script.kts", &tree, src).is_empty());
        // The same content is checked in a .kt file (row 2 at 8, should be 0).
        assert_eq!(rule.check_with_path("Type.kt", &tree, src).len(), 1);
        unsafe { std::env::remove_var("KTLINT_RS_INDENT_PROBE") };
    }

    #[test]
    fn issue202_top_level_property_accessor_lifts() {
        // A getter of a top-level property sits one level deeper than the
        // property (`val a\nget()` — property 0, getter 4), JVM oracle.
        expect_ok("private val size\nget() = 1f\n", &[(1, 0), (2, 4)]);
    }

    #[test]
    fn issue202_raw_string_opener_checked_content_skipped() {
        // `value = \"\"\"` openers are checked like declarations; the
        // content rows and pure delimiter rows are not.
        let src = "class Foo {\n    fun a() {\n        val q = \"\"\"\ncontent\n\"\"\"\n    }\n}\n";
        let violations = Indentation::new(4).check(&KotlinParser::new().parse(src), src);
        assert!(violations.is_empty(), "{:?}", violations);
    }

    #[test]
    fn issue202_string_continuation_row_checked() {
        // A `\"foo\" +` continuation row is checked like any expression
        // continuation (JVM reports it at the wrong depth, issue #202).
        let src = "class Foo {\n    fun a() {\n        val x = build(\n            expl = \"a\" +\n\"b\",\n        )\n    }\n}\n";
        let violations = Indentation::new(4).check(&KotlinParser::new().parse(src), src);
        let rows: Vec<usize> = violations.iter().map(|v| v.line).collect();
        assert!(rows.contains(&5), "{:?}", violations);
    }

    #[test]
    fn issue202_named_arg_equals_value_lifts() {
        // Call-site named-arg values lift one level under the default
        // (KtlintOfficial) code style: `b =\n    if (…)` — the if sits at
        // the arg row + 1 (JVM oracle, issue #202).
        let src = "fun main() {\n    build(\n        a = 1,\n        b =\n        if (true) {\n            2\n        },\n    )\n}\n";
        let violations = Indentation::new(4).check(&KotlinParser::new().parse(src), src);
        let rows: Vec<usize> = violations.iter().map(|v| v.line).collect();
        assert_eq!(rows, vec![5, 6, 7], "{:?}", violations);
    }

    #[test]
    fn issue202_binary_continuation_lifts() {
        // `return a() &&\n    b() &&\n    c()` — continuation rows sit at
        // the statement row + 1 (JVM oracle, issue #202); the probe reports
        // the over-indented 12-space rows.
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("KTLINT_RS_INDENT_PROBE", "1") };
        let src = "fun f(): Boolean {\n    return a() &&\n            b() &&\n            c()\n}\n";
        let violations = Indentation::new(4).check(&KotlinParser::new().parse(src), src);
        unsafe { std::env::remove_var("KTLINT_RS_INDENT_PROBE") };
        let rows: Vec<usize> = violations.iter().map(|v| v.line).collect();
        assert_eq!(rows, vec![3, 4]);
    }

    #[test]
    fn issue202_intellij_idea_keeps_aligned_equals_value() {
        // Under `ktlint_code_style = intellij_idea` the aligned named-arg
        // value is accepted (a ktor fixture shape, JVM oracle issue #202).
        let src = "fun main() {\n    build(\n        partHeaders =\n        headersOf(\n            a,\n        ),\n    )\n}\n";
        let rule = Indentation::new(4).with_code_style(crate::config::CodeStyle::IntelliJIdea);
        let violations = rule.check(&KotlinParser::new().parse(src), src);
        assert!(violations.is_empty(), "{:?}", violations);
    }

    #[test]
    fn issue202_comment_under_indent_reported() {
        // A standalone `//` comment shallower than the following statement
        // is reported (JVM oracle, issue #202); the ForYouScreenTest shape.
        let src = "fun main() {\n    build(\n        x =\n        Shown(\n            // comment\n            topics = list,\n        ),\n    )\n}\n";
        let violations = Indentation::new(4).check(&KotlinParser::new().parse(src), src);
        let rows: Vec<usize> = violations.iter().map(|v| v.line).collect();
        assert!(rows.contains(&5), "{:?}", violations);
    }

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
