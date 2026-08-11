//! Batch 4: Final ktlint parity rules (wrapping, block comment, etc.)
use crate::rules::{Rule, Violation};

pub struct CommentWrappingRule;
impl Rule for CommentWrappingRule {
    fn id(&self) -> &'static str {
        "standard:comment-wrapping"
    }
    /// Mirrors ktlint 1.8 CommentWrappingRule (main case): a block comment
    /// with code both before and after it on the same line is disallowed.
    /// `{ /* no-op */ }` is allowed.
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind().contains("comment") {
                let text = &source[node.start_byte()..node.end_byte()];
                if !text.starts_with("/*") {
                    let mut w = node.walk();
                    for c in node.children(&mut w) {
                        stack.push(c);
                    }
                    continue;
                }
                // Line of the comment; code before and after on the same line.
                let line_start = source[..node.start_byte()].rfind('\n').map_or(0, |i| i + 1);
                let line_end = source[node.end_byte()..]
                    .find('\n')
                    .map_or(source.len(), |i| node.end_byte() + i);
                let before = source[line_start..node.start_byte()].trim();
                let after = source[node.end_byte()..line_end].trim();
                let code_before = !before.is_empty();
                let code_after = !after.is_empty();
                // `{ /* no-op */ }` allowed.
                let lbrace_rbrace = before.ends_with('{') && after.starts_with('}');
                if (code_before && code_after) && !lbrace_rbrace {
                    let line = source[..node.start_byte()]
                        .bytes()
                        .filter(|&b| b == b'\n')
                        .count()
                        + 1;
                    violations.push(Violation {
                        file: String::new(),
                        line,
                        col: node.start_byte() - line_start + 1,
                        rule_id: self.id().into(),
                        message: "A block comment in between other elements on the same line is disallowed".into(),
                        auto_fixable: false,
                    });
                }
            }
            let mut w = node.walk();
            for c in node.children(&mut w) {
                stack.push(c);
            }
        }
        violations
    }
}

pub struct KdocWrappingRule;
impl Rule for KdocWrappingRule {
    fn id(&self) -> &'static str {
        "standard:kdoc-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        // Fail closed: the previous line-scan heuristic produced mass false
        // positives on real projects (verified against a live Spotless 8.8.0 +
        // ktlint 1.8.0 oracle with zero violations). A CST-aware implementation
        // must replace this before the rule can be re-enabled.
        Vec::new()
    }
}

// FunctionExpressionBodyRule removed — dead duplicate of phase3b_rules::FunctionExpressionBody

pub struct ParameterWrappingRule;
impl Rule for ParameterWrappingRule {
    fn id(&self) -> &'static str {
        "standard:parameter-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            let t = ln.trim();
            // Character count, not byte count — multi-byte text (e.g. CJK
            // strings) must not push a 115-char line over the 120 limit.
            if t.starts_with("fun ") && !t.contains('\n') && t.chars().count() > 120 {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Function signature too long, consider wrapping parameters".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct IfElseWrappingRule;
impl Rule for IfElseWrappingRule {
    fn id(&self) -> &'static str {
        "standard:if-else-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            if ln.trim() == "else"
                && i + 1 < l.len()
                && !l[i + 1].trim().starts_with("if")
                && !l[i + 1].trim().starts_with('{')
            {
                v.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "else body should be wrapped in braces".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct StatementWrappingRule;
impl Rule for StatementWrappingRule {
    fn id(&self) -> &'static str {
        "standard:statement-wrapping"
    }

    /// Mirrors ktlint 1.8 StatementWrappingRule: a block whose `{` (or `}`)
    /// shares a line with its first (last) statement needs a newline:
    ///   fun main() { println("hi") }   → 2 violations
    /// A `;` separating statements on one line also needs a newline:
    ///   val x = 1; println(x)          → 1 violation
    /// Excluded: empty blocks, single-line lambdas, single-line enums,
    /// trailing enum `;`.
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if matches!(
                node.kind(),
                "function_body"
                    | "control_structure_body"
                    | "class_body"
                    | "enum_class_body"
                    | "when_entry"
            ) {
                self.check_block(&node, source, &mut violations);
            }
            for i in (0..node.child_count()).rev() {
                if let Some(child) = node.child(i) {
                    stack.push(child);
                }
            }
        }
        self.check_semicolons(tree, source, &mut violations);
        violations
    }
}

impl StatementWrappingRule {
    fn check_semicolons(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        violations: &mut Vec<Violation>,
    ) {
        // Text scan for `;` used as a statement separator (skipping strings
        // and comments). ktlint's visitSemiColon: a `;` followed by code on
        // the same line needs a newline — except a trailing `;` in an
        // enum class body (`enum class E { A; }`).
        let bytes = source.as_bytes();
        let mut i = 0;
        let in_enum_tail = false;
        let mut enum_lines: Vec<usize> = Vec::new();
        // Identify enum class bodies (their trailing `;` is allowed).
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "enum_class_body" {
                for row in node.start_position().row..=node.end_position().row {
                    enum_lines.push(row);
                }
            }
            for k in (0..node.child_count()).rev() {
                if let Some(c) = node.child(k) {
                    stack.push(c);
                }
            }
        }
        let _ = in_enum_tail;
        while i < bytes.len() {
            // Advance over multi-byte characters so byte indexes stay on
            // char boundaries (e.g. `\u2192` in KDoc/strings).
            if bytes[i] >= 128 {
                let c = source[i..].chars().next().unwrap();
                i += c.len_utf8();
                continue;
            }
            // Skip strings and comments.
            if bytes[i] == b'"' {
                if source[i..].starts_with("\"\"\"") {
                    let close = source[i + 3..]
                        .find("\"\"\"")
                        .map_or(bytes.len(), |j| i + 3 + j + 3);
                    i = close;
                    continue;
                }
                // Skip a regular string, honoring escaped quotes.
                let mut j = i + 1;
                while j < bytes.len() && bytes[j] != b'"' {
                    if bytes[j] == b'\\' {
                        j += 2;
                    } else {
                        j += 1;
                    }
                }
                i = (j + 1).min(bytes.len());
                continue;
            }
            if bytes[i] == b'\'' {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j] != b'\'' {
                    if bytes[j] == b'\\' {
                        j += 2;
                    } else {
                        j += 1;
                    }
                }
                i = (j + 1).min(bytes.len());
                continue;
            }
            if source[i..].starts_with("//") {
                let nl = source[i..].find('\n').map_or(bytes.len(), |j| i + j);
                i = nl;
                continue;
            }
            if source[i..].starts_with("/*") {
                let close = source[i + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |j| i + 2 + j + 2);
                i = close;
                continue;
            }
            if bytes[i] == b';' {
                let row = source[..i].bytes().filter(|&b| b == b'\n').count();
                // Skip enum-class trailing semicolons.
                if enum_lines.contains(&row) {
                    i += 1;
                    continue;
                }
                let after = &source[i + 1..];
                let next = after
                    .chars()
                    .next()
                    .map(|c| c != '\n' && !c.is_whitespace())
                    .unwrap_or(false)
                    || after
                        .chars()
                        .take_while(|c| c.is_whitespace())
                        .next()
                        .is_some_and(|c| c != '\n')
                        && after.trim_start().chars().next().is_some_and(|c| c != '\n');
                if next && !after.trim_start().starts_with("//") {
                    violations.push(self.v(i + 1, source, "Missing newline after ';'"));
                }
            }
            i += 1;
        }
    }

    fn check_block(&self, node: &tree_sitter::Node, source: &str, violations: &mut Vec<Violation>) {
        let start = node.start_byte();
        let end = node.end_byte();
        let text = &source[start..end];

        // Single-line lambda and single-line enum bodies are allowed
        // (`val f = { a -> a }`, `enum class E { A, B }`).
        if node.kind() == "enum_class_body" && node.start_position().row == node.end_position().row
        {
            return;
        }
        // tree-sitter-kotlin parses `by remember(...) { expr }` as a nested
        // function_declaration with a brace body (the lambda's `{` becomes the
        // function_body). A real `fun` declaration has its parameter list
        // immediately before the function_body; the mis-parsed one does not.
        if node.kind() == "function_body" {
            // The function_body must directly follow the parameter list —
            // only whitespace in between. tree-sitter-kotlin mis-parses
            // `var x by remember(...) { expr }` so the lambda's `{` becomes
            // the enclosing function's body, with code (the `by` delegation)
            // sitting between the params and the `{`.
            let is_real_fun = node
                .parent()
                .filter(|p| p.kind() == "function_declaration")
                .is_some_and(|p| {
                    let mut pw = p.walk();
                    let kids: Vec<tree_sitter::Node> = p.children(&mut pw).collect();
                    for (i, kid) in kids.iter().enumerate() {
                        if kid == node && i > 0 {
                            let prev = kids[i - 1];
                            if prev.kind() == "function_value_parameters" {
                                // tree-sitter bug: `by remember(...) { }`
                                // swallows the delegation (and its `{`) into
                                // the parameter list; a real parameter list
                                // never contains `{`.
                                let params_text = &source[prev.start_byte()..prev.end_byte()];
                                if params_text.contains('{') {
                                    return false;
                                }
                                return source[prev.end_byte()..node.start_byte()]
                                    .chars()
                                    .all(|c| c.is_whitespace());
                            }
                        }
                    }
                    false
                });
            if !is_real_fun {
                return;
            }
        }

        // A block's `{` must be the first code character of the node (or,
        // for when-entries, follow the `->`). Anything else — expression
        // bodies, `-> let { … }` lambdas, `{` inside string templates — is
        // not a brace block.
        let (lbrace, rbrace) = if node.kind() == "when_entry" {
            let Some(arrow) = text.find("->") else { return };
            let after = text[arrow + 2..].trim_start();
            if !after.starts_with('{') {
                return;
            }
            let lbrace_rel = arrow + 2 + (text[arrow + 2..].len() - after.len());
            let Some(rbrace_rel) = text[lbrace_rel + 1..].rfind('}') else {
                return;
            };
            (start + lbrace_rel, start + lbrace_rel + 1 + rbrace_rel)
        } else {
            let trimmed = text.trim_start();
            if !trimmed.starts_with('{') {
                return;
            }
            let lbrace_rel = text.len() - trimmed.len();
            let Some(rbrace_rel) = text[lbrace_rel + 1..].rfind('}') else {
                return;
            };
            (start + lbrace_rel, start + lbrace_rel + 1 + rbrace_rel)
        };
        // Empty block `{}` and comment-only blocks `{ /* no-op */ }` are allowed.
        if next_code_token(source, lbrace + 1, rbrace).is_none() {
            return;
        }

        // The first code token after `{` — ktlint's `nextCodeLeaf` (skips
        // whitespace and comments).
        if let Some(next) = next_code_token(source, lbrace + 1, rbrace) {
            if !source[lbrace..next.0].contains('\n')
                && node.start_position().row == source[..next.0].matches('\n').count()
            {
                // `{` and the first statement share a line.
                violations.push(self.v(next.0, source, "Missing newline after '{'"));
            }
        }
        // The last code token before `}`.
        if let Some(prev) = prev_code_token(source, lbrace + 1, rbrace) {
            if !source[prev.1..rbrace].contains('\n') {
                violations.push(self.v(rbrace, source, "Missing newline before '}'"));
            }
        }
    }

    fn v(&self, pos: usize, source: &str, message: &str) -> Violation {
        let line = source[..pos].bytes().filter(|&b| b == b'\n').count() + 1;
        let line_start = source[..pos].rfind('\n').map_or(0, |i| i + 1);
        Violation {
            file: String::new(),
            line,
            col: pos - line_start + 1,
            rule_id: self.id().into(),
            message: message.into(),
            auto_fixable: true,
        }
    }
}

/// Byte range of the first non-whitespace, non-comment token in `[start, end)`.
fn next_code_token(source: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let mut i = start;
    while i < end {
        let c = source[i..].chars().next()?;
        if c.is_whitespace() {
            i += c.len_utf8();
            continue;
        }
        if source[i..].starts_with("//") || source[i..].starts_with("/*") {
            let nl = source[i..].find('\n').map_or(end, |j| i + j);
            if source[i..].starts_with("/*") {
                let close = source[i + 2..].find("*/").map_or(end, |j| i + 2 + j + 2);
                i = close;
            } else {
                i = nl;
            }
            continue;
        }
        return Some((i, i + c.len_utf8()));
    }
    None
}

/// Byte range of the last non-whitespace, non-comment token in `[start, end)`.
fn prev_code_token(source: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let chars = source[start..end].char_indices().rev();
    for (rel, c) in chars {
        let i = start + rel;
        if c.is_whitespace() {
            continue;
        }
        // Simple backward skip for comments (rare before `}`).
        if source[..i].ends_with("//") || source[..i].ends_with("*/") {
            continue;
        }
        return Some((i, i + c.len_utf8()));
    }
    None
}

pub struct ChainMethodContinuationRule;
impl Rule for ChainMethodContinuationRule {
    fn id(&self) -> &'static str {
        "standard:chain-method-continuation"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        fn indent(line: &str) -> usize {
            line.len() - line.trim_start().len()
        }
        // Every link of a chained call must align: the chain's start
        // expression line indent + one level. The old implementation
        // reported *every* '.' line regardless of alignment (thousands of
        // false positives on the consumer fixtures).
        let mut i = 0usize;
        while i < l.len() {
            let t = l[i].trim_start();
            if !t.starts_with('.') || t.starts_with("...") {
                i += 1;
                continue;
            }
            // The chain's alignment: every link sits at the same indent as
            // the chain start expression + one level. When the previous
            // lines are themselves links (`).setHeader(…`, `}`), the
            // expectation is their own indent (chain members align).
            let mut j = i;
            let mut prev_link: Option<usize> = None;
            while j > 0 {
                let pt = l[j - 1].trim_start();
                if pt.starts_with('.') {
                    prev_link = Some(j - 1);
                    break;
                }
                if (pt.starts_with(')') || pt.starts_with('}')) && pt.contains('.') {
                    // `).setHeader(…` / `}.also { … }` — a link that starts
                    // with its closing delimiter; it defines the alignment.
                    prev_link = Some(j - 1);
                    break;
                }
                if pt.starts_with(')') || pt.starts_with('}') {
                    j -= 1;
                } else {
                    break;
                }
            }
            let want = match prev_link {
                Some(link) => indent(l[link]),
                None => {
                    let start_line = if j > 0 { j - 1 } else { j };
                    if start_line == i || l[start_line].trim().is_empty() {
                        i += 1;
                        continue;
                    }
                    indent(l[start_line]).saturating_add(2)
                }
            };
            // Walk forward through the chain's '.' links, starting at the
            // current line (never behind it).
            let mut k = i;
            while k < l.len() && l[k].trim_start().starts_with('.') {
                if !l[k].trim_start().starts_with("...") && indent(l[k]) != want {
                    v.push(Violation {
                        file: String::new(),
                        line: k + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Chain continuation '.' should align with previous line".into(),
                        auto_fixable: true,
                    });
                }
                k += 1;
            }
            i = k;
        }
        v
    }
}

pub struct MultilineLoopRule;
impl Rule for MultilineLoopRule {
    fn id(&self) -> &'static str {
        "standard:multiline-loop"
    }
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        // Fail closed: the previous line-scan heuristic produced mass false
        // positives on real projects (verified against a live Spotless 8.8.0 +
        // ktlint 1.8.0 oracle with zero violations). A CST-aware implementation
        // must replace this before the rule can be re-enabled.
        Vec::new()
    }
}
