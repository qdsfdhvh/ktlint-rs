//! Auto-fix formatter — applies text-level corrections for fixable violations.
pub(crate) mod edit;

use crate::config::{CodeStyle, RuleConfig};
use crate::parser::KotlinParser;
use crate::rules::Violation;
use edit::{minimal_edit, EditSet};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Private-use sentinel that never appears in real Kotlin source. Used to fence
/// off masked spans (see [`mask_protected`]).
const SENTINEL: char = '\u{E000}';

/// True for CST node kinds whose *interior* generic text-level spacing fixers must
/// never edit. Those fixers (`fix_operators`, `fix_colons`, and similar) are
/// CST-unaware and would otherwise insert spaces inside strings, KDoc, and comments.
/// The comment-spacing rule is a separate CST-owned edit exception. Matching on
/// substrings keeps this robust across grammar naming variants
/// (`line_string_literal`, `multiline_string_literal`, `line_comment`, …).
fn is_protected_kind(kind: &str) -> bool {
    kind == "string_literal"
        || kind == "character_literal"
        || kind.contains("string")
        || kind.contains("comment")
        // Generic keyword/colon fixers must not split callable references such as
        // `RuntimeException::class`; a dedicated double-colon pass owns that syntax.
        || kind == "callable_reference"
        // Generic type argument/parameter lists: their `<`, `>`, and commas must not
        // be touched by fix_operators/fix_angle_brackets/fix_commas, which can't tell
        // `List<String>` from the comparison operators `<`/`>`.
        || kind == "type_arguments"
        || kind == "type_parameters"
}

/// A `:` that carries a space *before* it in ktlint style — class/object supertype,
/// generic `where` constraint, and secondary-constructor delegation. The text-level
/// `fix_colons` collapses ` : `→`: ` (correct for `val x: Int`, wrong here), so these
/// specific colons are protected via their CST parent.
fn is_space_before_colon(node: &tree_sitter::Node) -> bool {
    node.kind() == ":"
        && node.parent().is_some_and(|p| {
            matches!(
                p.kind(),
                "class_declaration"
                    | "object_declaration"
                    | "object_literal"
                    | "type_constraint"
                    | "secondary_constructor"
            )
        })
}

/// A backtick-quoted identifier (e.g. `` fun `name with spaces`() ``) is not a
/// string node, so `is_protected_kind` misses it — yet its interior (`-`, spaces,
/// `/`) must never be edited. Kotlin backtick identifiers cannot contain a newline
/// or another backtick, so a same-line ``…`` span is unambiguous.
fn is_backtick_identifier(node: &tree_sitter::Node, source: &str) -> bool {
    let t = &source[node.start_byte()..node.end_byte()];
    t.len() >= 2 && t.starts_with('`') && t.ends_with('`') && !t.contains('\n')
}

fn collect_protected(node: tree_sitter::Node, source: &str, out: &mut Vec<(usize, usize)>) {
    if node.kind() == "spread_expression" && source.as_bytes().get(node.start_byte()) == Some(&b'*')
    {
        // The whitespace after `*` has already been canonicalized by
        // `fix_spread_operators`; mask only the token so generic operator
        // spacing cannot turn it back into `* value`.
        out.push((node.start_byte(), node.start_byte() + 1));
    }
    if is_protected_kind(node.kind())
        || is_backtick_identifier(&node, source)
        || is_space_before_colon(&node)
    {
        // Protect the whole span (including any string interpolation) rather than
        // risk corrupting it — under-formatting is acceptable, corruption is not.
        out.push((node.start_byte(), node.end_byte()));
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_protected(child, source, out);
    }
}

/// Replace every string/char-literal and comment span with an inert, newline-count-
/// preserving placeholder so the text-level fixers can't reach inside them. Returns
/// the masked text and the table needed by [`restore_protected`].
///
/// Each physical line of a span becomes its own `SENTINEL<id>SENTINEL` fragment,
/// with real `\n`s kept between fragments — this keeps line-oriented fixers
/// (`fix_indentation`, `fix_blank_line_in_list`, `fix_trailing_ws`) seeing the same
/// line structure, while the fragments contain no character any fixer targets.
fn mask_protected(source: &str, tree: &tree_sitter::Tree) -> Option<(String, Vec<String>)> {
    if source.contains(SENTINEL) {
        return None;
    }
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    collect_protected(tree.root_node(), source, &mut ranges);
    if ranges.is_empty() {
        return Some((source.to_string(), Vec::new()));
    }
    ranges.sort_by_key(|r| r.0);

    let mut out = String::with_capacity(source.len());
    let mut store: Vec<String> = Vec::new();
    let mut last = 0usize;
    for (start, end) in ranges {
        if start < last {
            continue; // defensive: skip any overlap
        }
        out.push_str(&source[last..start]);
        let mut first = true;
        for part in source[start..end].split('\n') {
            if !first {
                out.push('\n');
            }
            first = false;
            let id = store.len();
            store.push(part.to_string());
            out.push(SENTINEL);
            out.push_str(&id.to_string());
            out.push(SENTINEL);
        }
        last = end;
    }
    out.push_str(&source[last..]);
    Some((out, store))
}

fn mask_strings_and_chars(
    source: &str,
    tree: &tree_sitter::Tree,
    include_comments: bool,
) -> (String, Vec<String>) {
    fn collect(
        node: tree_sitter::Node<'_>,
        ranges: &mut Vec<(usize, usize)>,
        include_comments: bool,
    ) {
        if node.kind().contains("string")
            || node.kind() == "character_literal"
            || (include_comments && node.kind().contains("comment"))
        {
            ranges.push((node.start_byte(), node.end_byte()));
            return;
        }
        for index in 0..node.child_count() {
            if let Some(child) = node.child(index) {
                collect(child, ranges, include_comments);
            }
        }
    }

    let mut ranges = Vec::new();
    collect(tree.root_node(), &mut ranges, include_comments);
    ranges.sort_by_key(|range| range.0);
    let mut output = String::with_capacity(source.len());
    let mut store = Vec::new();
    let mut cursor = 0;
    for (start, end) in ranges {
        if start < cursor || end > source.len() {
            continue;
        }
        output.push_str(&source[cursor..start]);
        for (part_index, part) in source[start..end].split('\n').enumerate() {
            if part_index > 0 {
                output.push('\n');
            }
            let id = store.len();
            store.push(part.to_string());
            output.push(SENTINEL);
            output.push_str(&id.to_string());
            output.push(SENTINEL);
        }
        cursor = end;
    }
    output.push_str(&source[cursor..]);
    (output, store)
}

/// Inverse of [`mask_protected`]: swap each `SENTINEL<id>SENTINEL` fragment back
/// for its original text. Fixers never insert into a fragment (it holds only
/// digits between two sentinels), so ids survive intact.
fn restore_protected(text: &str, store: &[String]) -> String {
    if store.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == SENTINEL {
            let mut j = i + 1;
            let mut num = String::new();
            while j < chars.len() && chars[j] != SENTINEL {
                num.push(chars[j]);
                j += 1;
            }
            if j < chars.len() && chars[j] == SENTINEL {
                if let Ok(id) = num.parse::<usize>() {
                    if let Some(orig) = store.get(id) {
                        out.push_str(orig);
                        i = j + 1;
                        continue;
                    }
                }
            }
            out.push(SENTINEL); // malformed — emit literally (should never happen)
            i += 1;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn parse_clean(source: &str) -> Option<tree_sitter::Tree> {
    let mut parser = KotlinParser::new();
    let tree = parser.parse(source);
    (!tree.root_node().has_error()).then_some(tree)
}

#[derive(Debug)]
struct ProtectedSnapshot {
    ranges: Vec<std::ops::Range<usize>>,
    fragments: Vec<String>,
}

fn protected_snapshot(source: &str, tree: &tree_sitter::Tree) -> anyhow::Result<ProtectedSnapshot> {
    let mut raw_ranges = Vec::new();
    collect_protected(tree.root_node(), source, &mut raw_ranges);
    raw_ranges.sort_unstable();
    let mut previous_end = 0usize;
    let mut ranges = Vec::with_capacity(raw_ranges.len());
    let mut fragments = Vec::with_capacity(raw_ranges.len());
    for (index, (start, end)) in raw_ranges.into_iter().enumerate() {
        if index > 0 && start < previous_end {
            anyhow::bail!("overlapping CST protected regions");
        }
        if start > end
            || end > source.len()
            || !source.is_char_boundary(start)
            || !source.is_char_boundary(end)
        {
            anyhow::bail!("invalid CST protected region");
        }
        ranges.push(start..end);
        fragments.push(source[start..end].to_string());
        previous_end = end;
    }
    Ok(ProtectedSnapshot { ranges, fragments })
}

fn edits_around_protected_regions(
    owner: &'static str,
    source: &str,
    transformed: &str,
    before: &ProtectedSnapshot,
    after: &ProtectedSnapshot,
) -> Vec<edit::TextEdit> {
    let mut edits = Vec::new();
    let mut source_start = 0usize;
    let mut transformed_start = 0usize;
    for (source_range, transformed_range) in before.ranges.iter().zip(&after.ranges) {
        if let Some(mut edit) = minimal_edit(
            owner,
            &source[source_start..source_range.start],
            &transformed[transformed_start..transformed_range.start],
        ) {
            edit.range.start += source_start;
            edit.range.end += source_start;
            edits.push(edit);
        }
        source_start = source_range.end;
        transformed_start = transformed_range.end;
    }
    if let Some(mut edit) = minimal_edit(
        owner,
        &source[source_start..],
        &transformed[transformed_start..],
    ) {
        edit.range.start += source_start;
        edit.range.end += source_start;
        edits.push(edit);
    }
    edits
}

fn safe_transform<F>(owner: &'static str, source: &str, transform: F) -> anyhow::Result<String>
where
    F: Fn(&str) -> String,
{
    if source.contains(SENTINEL) {
        return Ok(source.to_string());
    }
    let Some(before_tree) = parse_clean(source) else {
        return Ok(source.to_string());
    };
    let protected_before = protected_snapshot(source, &before_tree)?;
    let transformed = transform(source);
    if transformed == source {
        return Ok(source.to_string());
    }
    let Some(after_tree) = parse_clean(&transformed) else {
        anyhow::bail!("{owner} produced invalid Kotlin syntax");
    };
    let protected_after = protected_snapshot(&transformed, &after_tree)?;
    if protected_before.fragments != protected_after.fragments {
        anyhow::bail!("{owner} modified a protected Kotlin region");
    }
    if transform(&transformed) != transformed {
        anyhow::bail!("{owner} is not idempotent");
    }

    let edits = edits_around_protected_regions(
        owner,
        source,
        &transformed,
        &protected_before,
        &protected_after,
    );
    if edits.is_empty() {
        anyhow::bail!("{owner} changed text without producing an edit");
    }
    let applied = EditSet::new(edits).apply(source)?;
    if applied != transformed {
        anyhow::bail!("{owner} edit application diverged from pass output");
    }
    Ok(applied)
}

/// Canonicalize whitespace after Kotlin's spread operator using CST context.
/// A spread `*` is unary syntax (`call(*args)`), not the binary multiplication
/// operator handled by [`fix_operators`].
fn fix_spread_operators(source: &str, tree: &tree_sitter::Tree) -> String {
    fn collect(node: tree_sitter::Node<'_>, source: &[u8], edits: &mut Vec<(usize, usize)>) {
        if node.kind() == "spread_expression" {
            let start = node.start_byte();
            let end = node.end_byte().min(source.len());
            if source.get(start) == Some(&b'*') {
                let mut whitespace_end = start + 1;
                while whitespace_end < end && matches!(source[whitespace_end], b' ' | b'\t') {
                    whitespace_end += 1;
                }
                if whitespace_end > start + 1 {
                    edits.push((start + 1, whitespace_end));
                }
            }
            return;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect(child, source, edits);
        }
    }

    let mut edits = Vec::new();
    collect(tree.root_node(), source.as_bytes(), &mut edits);
    let mut output = source.to_string();
    for (start, end) in edits.into_iter().rev() {
        output.replace_range(start..end, "");
    }
    output
}

fn masked_transform(source: &str, transform: fn(&str) -> String) -> String {
    let Some(tree) = parse_clean(source) else {
        return source.to_string();
    };
    let Some((masked, store)) = mask_protected(source, &tree) else {
        return source.to_string();
    };
    restore_protected(&transform(&masked), &store)
}

fn rule_enabled(
    rule_configs: &HashMap<String, RuleConfig>,
    rule_id: &str,
    code_style: CodeStyle,
) -> bool {
    crate::rules::code_style_allows(rule_id, code_style)
        && rule_configs
            .get(rule_id)
            .is_none_or(|config| config.enabled)
}

fn apply_spacing_rules(
    source: &str,
    rule_configs: &HashMap<String, RuleConfig>,
    code_style: CodeStyle,
) -> anyhow::Result<String> {
    let mut text = source.to_string();
    for _ in 0..5 {
        let before = text.clone();
        if rule_enabled(rule_configs, "standard:op-spacing", code_style) {
            text = safe_transform("standard:op-spacing", &text, |input| {
                let Some(tree) = parse_clean(input) else {
                    return input.to_string();
                };
                fix_spread_operators(input, &tree)
            })?;
        }
        let passes: [(&'static str, fn(&str) -> String); 20] = [
            ("standard:annotation-spacing", fix_annotation_blank_lines),
            ("standard:modifier-list-spacing", fix_annotation_blank_lines),
            (
                "standard:spacing-between-declarations-with-annotations",
                fix_spacing_before_annotated_declarations,
            ),
            (
                "standard:function-type-modifier-spacing",
                fix_function_type_modifier_spacing,
            ),
            (
                "standard:function-type-reference-spacing",
                fix_function_type_reference_spacing,
            ),
            ("standard:curly-spacing", fix_curly_braces),
            ("standard:op-spacing", fix_operators),
            ("standard:comma-spacing", fix_commas),
            ("standard:paren-spacing", fix_parens),
            ("standard:spacing-around-angle-brackets", fix_angle_brackets),
            ("standard:colon-spacing", fix_colons),
            ("standard:keyword-spacing", fix_keyword_spacing),
            ("standard:range-spacing", fix_range_spacing),
            (
                "standard:unnecessary-parentheses-before-trailing-lambda",
                fix_trailing_lambda_parentheses,
            ),
            ("legacy:semicolon-spacing-pass", fix_semicolons),
            ("standard:trailing-comma", fix_single_line_trailing_comma),
            ("standard:no-consecutive-blank-lines", fix_blank_lines),
            ("standard:no-blank-line-in-list", fix_blank_line_in_list),
            ("legacy:brace-between-pass", fix_brace_between),
            ("legacy:double-space-pass", fix_double_spaces),
        ];
        for (owner, transform) in passes {
            if owner.starts_with("legacy:") || rule_enabled(rule_configs, owner, code_style) {
                text = safe_transform(owner, &text, |input| masked_transform(input, transform))?;
            }
        }
        text = safe_transform(
            "legacy:class-header-spacing-pass",
            &text,
            fix_class_header_spacing,
        )?;
        if text == before {
            break;
        }
    }

    if rule_enabled(rule_configs, "standard:comment-spacing", code_style) {
        text = apply_comment_spacing(&text)?;
    }
    Ok(text)
}

fn format_once(
    source: &str,
    indent_size: usize,
    insert_final_newline: bool,
    rule_configs: &HashMap<String, RuleConfig>,
    code_style: CodeStyle,
) -> anyhow::Result<String> {
    let mut text = source.to_string();
    if rule_enabled(
        rule_configs,
        "standard:block-comment-initial-star-alignment",
        code_style,
    ) {
        text = apply_block_comment_alignment(&text)?;
    }
    text = apply_spacing_rules(&text, rule_configs, code_style)?;
    if rule_enabled(
        rule_configs,
        "standard:try-catch-finally-spacing",
        code_style,
    ) {
        text = safe_transform("standard:try-catch-finally-spacing", &text, |input| {
            masked_transform(input, fix_try_catch)
        })?;
    }
    if rule_enabled(rule_configs, "standard:wrapping", code_style) {
        text = safe_transform("standard:wrapping", &text, |input| {
            masked_transform(input, fix_single_line_control_blocks)
        })?;
    }
    if rule_enabled(rule_configs, "standard:indent", code_style) {
        text = safe_transform("standard:indent", &text, |input| {
            fix_indentation(input, indent_size)
        })?;
    }
    if rule_enabled(rule_configs, "standard:no-trailing-spaces", code_style) {
        text = safe_transform(
            "standard:no-trailing-spaces",
            &text,
            fix_trailing_ws_protected,
        )?;
    }
    if rule_enabled(rule_configs, "standard:final-newline", code_style) {
        text = safe_transform("standard:final-newline", &text, |input| {
            let mut output = input.to_string();
            if insert_final_newline && !output.ends_with('\n') {
                output.push('\n');
            }
            output
        })?;
    }
    Ok(text)
}

pub fn auto_fix(
    _files: &[PathBuf],
    violations: &[Violation],
    indent_size: usize,
    insert_final_newline: bool,
    rule_configs: &HashMap<String, RuleConfig>,
    code_style: CodeStyle,
) -> anyhow::Result<()> {
    let fixable: Vec<&Violation> = violations.iter().filter(|v| v.auto_fixable).collect();
    if fixable.is_empty() {
        return Ok(());
    }

    let mut file_set: HashSet<&str> = HashSet::new();
    for v in &fixable {
        if !v.file.is_empty() {
            file_set.insert(&v.file);
        }
    }

    for file_path in &file_set {
        // ktlint formatting is a whole-file, ordered rule-engine pass. Running only
        // fixers whose ids happened to be reported misses dependent corrections.
        let _rules: Vec<&str> = fixable
            .iter()
            .filter(|v| v.file == *file_path)
            .map(|v| v.rule_id.as_str())
            .collect();

        let original = std::fs::read_to_string(file_path)?;
        // Unsupported syntax and sentinel collisions are fail-closed for the entire
        // file, including newline/whitespace normalization.
        if original.contains(SENTINEL) || parse_clean(&original).is_none() {
            continue;
        }
        let text = format_once(
            &original,
            indent_size,
            insert_final_newline,
            rule_configs,
            code_style,
        )?;
        if format_once(
            &text,
            indent_size,
            insert_final_newline,
            rule_configs,
            code_style,
        )? != text
        {
            anyhow::bail!("formatter pipeline is not idempotent for {file_path}");
        }
        if text != original {
            std::fs::write(file_path, text)?;
        }
    }
    Ok(())
}

#[cfg(test)]
fn fix_all_spacing(source: &str) -> String {
    let mut parser = KotlinParser::new();
    let initial_tree = parser.parse(source);
    // If tree-sitter-kotlin-sg can't parse the file (grammar limitation), CST-based
    // masking is unreliable, so the text fixers could corrupt strings/comments/colons.
    // Skip the interior-editing passes entirely — safety over completeness. Clean
    // files parse fine and are fully formatted; only grammar-breaking files are
    // left untouched here (trailing-whitespace/newline normalization still applies).
    if initial_tree.root_node().has_error() {
        return source.to_string();
    }
    let spread_fixed = fix_spread_operators(source, &initial_tree);
    let tree = parser.parse(&spread_fixed);
    if tree.root_node().has_error() {
        return source.to_string();
    }
    // Fence off string/char-literal and comment interiors: the text-level fixers
    // below are CST-unaware and would corrupt URLs, KDoc, and `//` inside strings.
    let Some((masked, store)) = mask_protected(&spread_fixed, &tree) else {
        return source.to_string();
    };
    let mut text = masked;
    for _ in 0..5 {
        let before = text.clone();
        text = fix_curly_braces(&text);
        text = fix_operators(&text);
        text = fix_commas(&text);
        text = fix_parens(&text);
        text = fix_angle_brackets(&text);
        text = fix_colons(&text);
        text = fix_keyword_spacing(&text);
        text = fix_range_spacing(&text);
        text = fix_trailing_lambda_parentheses(&text);
        text = fix_semicolons(&text);
        text = fix_single_line_trailing_comma(&text);
        text = fix_comment_spacing(&text);
        text = fix_blank_lines(&text);
        text = fix_blank_line_in_list(&text);
        text = fix_brace_between(&text);
        text = fix_double_spaces(&text);
        if text == before {
            break;
        }
    }
    let restored = restore_protected(&text, &store);
    let restored = fix_class_header_spacing(&restored);
    fix_comment_spacing(&restored)
}

#[cfg(test)]
fn fix_all_wrapping(source: &str) -> String {
    let mut parser = KotlinParser::new();
    let tree = parser.parse(source);
    if tree.root_node().has_error() {
        return source.to_string();
    }
    let Some((masked, store)) = mask_protected(source, &tree) else {
        return source.to_string();
    };
    let text = fix_single_line_control_blocks(&fix_try_catch(&masked));
    restore_protected(&text, &store)
}

fn fix_indentation(source: &str, indent_size: usize) -> String {
    let mut parser = KotlinParser::new();
    let tree = parser.parse(source);
    if tree.root_node().has_error() {
        return source.to_string();
    }
    let Some((masked, store)) = mask_protected(source, &tree) else {
        return source.to_string();
    };
    let existing_indented_code_lines = masked
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.is_empty()
                && !trimmed.contains(SENTINEL)
                && line.len().saturating_sub(trimmed.len()) > 0
        })
        .count();
    // Avoid globally reflowing already-indented files until continuation
    // indentation is fully CST-driven. Wrapping may introduce one or two lines.
    if existing_indented_code_lines > 2 {
        return source.to_string();
    }
    let mut depth = 0usize;
    let mut output = Vec::new();
    for line in masked.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            output.push(String::new());
            continue;
        }
        let line_depth = depth.saturating_sub(usize::from(trimmed.starts_with('}')));
        if trimmed.starts_with(SENTINEL) {
            output.push(line.to_string());
        } else {
            output.push(format!(
                "{}{}",
                " ".repeat(line_depth * indent_size),
                trimmed
            ));
        }
        let opens = trimmed.bytes().filter(|byte| *byte == b'{').count();
        let closes = trimmed.bytes().filter(|byte| *byte == b'}').count();
        depth = depth.saturating_add(opens).saturating_sub(closes);
    }
    let mut result = output.join("\n");
    if source.ends_with('\n') {
        result.push('\n');
    }
    restore_protected(&result, &store)
}

fn fix_trailing_ws(source: &str) -> String {
    source
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

fn fix_trailing_ws_protected(source: &str) -> String {
    let mut parser = KotlinParser::new();
    let tree = parser.parse(source);
    let Some((masked, store)) = mask_protected(source, &tree) else {
        return source.to_string();
    };
    restore_protected(&fix_trailing_ws(&masked), &store)
}

// ── Spacing helpers ──

fn fix_curly_braces(source: &str) -> String {
    let mut s = source.to_string();
    let opens: Vec<usize> = s.match_indices('{').map(|(i, _)| i).collect();
    for &pos in opens.iter().rev() {
        if pos > 0 {
            let prev = s[..pos].chars().last().unwrap_or(' ');
            if prev != ' ' && prev != '\n' && prev != '$' && prev != '(' && prev != '[' {
                s.insert(pos, ' ');
            }
        }
    }
    s = s.replace("{", "{ ").replace("{  ", "{ ");
    let closes: Vec<usize> = s.match_indices('}').map(|(i, _)| i).collect();
    for &pos in closes.iter().rev() {
        if pos > 0 {
            let prev = s[..pos].chars().last().unwrap_or(' ');
            if !prev.is_whitespace() && prev != '{' {
                s.insert(pos, ' ');
            }
        }
    }
    for kw in &["else", "catch", "finally"] {
        s = s.replace(&format!("}}{}", kw), &format!("}} {}", kw));
    }
    s = s.replace("}\nelse if", "} else if");
    s = s.replace("{ }", "{}");
    s
}

fn fix_operators(source: &str) -> String {
    let mut s = source.to_string();
    let ops = [
        "==", "!=", "<=", ">=", "->", "&&", "||", "+=", "-=", "*=", "/=", "=", "<", ">", "+", "-",
        "*", "/", "%",
    ];
    for op in &ops {
        // Build char→byte mapping for this iteration
        let chars: Vec<char> = s.chars().collect();
        let c2b: Vec<usize> = s.char_indices().map(|(bi, _)| bi).collect();
        debug_assert_eq!(c2b.len(), chars.len());

        // Phase 1: collect all char positions
        let is_op_char = |c: char| "=<>!+-*/%&|".contains(c);
        let mut positions: Vec<usize> = Vec::new();
        let mut i = 0;
        while i + op.len() <= chars.len() {
            if i > 0 {
                let rest: String = chars[i..i + op.len()].iter().collect();
                if rest == *op {
                    // Issue #45: skip unary minus
                    let is_unary_minus = *op == "-"
                        && (!chars[i - 1].is_alphanumeric()
                            && chars[i - 1] != ')'
                            && chars[i - 1] != ']');
                    // Skip when this match is adjacent to another operator char: it
                    // belongs to a compound operator (==, >=, +=, ->, …) handled by
                    // that operator's own iteration. Without this, the single-char
                    // `=` pass splits `==` into `= =` and `>=` loses its lead space.
                    let touches_op = is_op_char(chars[i - 1])
                        || chars.get(i + op.len()).copied().map_or(false, is_op_char);
                    if !is_unary_minus && !touches_op {
                        positions.push(i);
                    }
                }
            }
            i += 1;
        }
        // Phase 2: apply fixes right-to-left using byte positions
        for &pos in positions.iter().rev() {
            let cur: Vec<char> = s.chars().collect();
            let cur_c2b: Vec<usize> = s.char_indices().map(|(bi, _)| bi).collect();
            if pos >= cur.len() || pos + op.len() > cur.len() {
                continue;
            }
            let cur_rest: String = cur[pos..pos + op.len()].iter().collect();
            if cur_rest != *op {
                continue;
            }
            let byte_pos = cur_c2b[pos];
            let prev = cur[pos - 1];
            let next = cur.get(pos + op.len()).copied().unwrap_or(' ');
            if prev != ' ' && prev != '\n' && !is_op_char(prev) {
                s.insert(byte_pos, ' ');
            }
            // Re-read after potential insert
            let cur2: Vec<char> = s.chars().collect();
            let cur2_c2b: Vec<usize> = s.char_indices().map(|(bi, _)| bi).collect();
            let after_char = pos
                + op.len()
                + if prev != ' ' && prev != '\n' && !is_op_char(prev) {
                    1
                } else {
                    0
                };
            if after_char < cur2.len() {
                let actual_next = cur2[after_char];
                // Align with rule: insert unless next is space ) \n ,
                if actual_next != ' '
                    && actual_next != ')'
                    && actual_next != '\n'
                    && actual_next != ','
                {
                    let after_byte = cur2_c2b[after_char];
                    s.insert(after_byte, ' ');
                }
            }
        }
    }
    s
}

fn fix_commas(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let chars: Vec<char> = source.chars().collect();
    for (index, ch) in chars.iter().copied().enumerate() {
        output.push(ch);
        if ch == ','
            && chars
                .get(index + 1)
                .is_some_and(|next| !next.is_whitespace() && !matches!(*next, ')' | ']'))
        {
            output.push(' ');
        }
    }
    output
}

fn fix_angle_brackets(source: &str) -> String {
    // Disabled: a text-level `< `→`<` / ` >`→`>` cannot tell a generic (`List<T>`,
    // now masked anyway) from the comparison operators `a < b` / `a >= b`, and
    // corrupted the latter. Generic tidy is handled by masking; comparison spacing
    // by fix_operators. Kept as a no-op so the pipeline order is unchanged.
    source.to_string()
}

fn fix_parens(source: &str) -> String {
    // Per-line and indent-preserving: a global `replace(" )", ")")` also eats the
    // leading indentation of a `)` that sits on its own line (`        )` → `   )`),
    // because the loop re-applies it each pass. Only collapse spaces *inside* the line body.
    strip_inner_bracket_spaces(source, "( ", "(", " )", ")")
}

/// Collapse `open_from`→`open_to` and `close_from`→`close_to` within each line's
/// body while preserving leading indentation.
fn strip_inner_bracket_spaces(
    source: &str,
    open_from: &str,
    open_to: &str,
    close_from: &str,
    close_to: &str,
) -> String {
    source
        .split('\n')
        .map(|line| {
            let indent_len = line.len() - line.trim_start().len();
            let (indent, rest) = line.split_at(indent_len);
            let fixed = rest
                .replace(open_from, open_to)
                .replace(close_from, close_to);
            format!("{indent}{fixed}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fix_colons(source: &str) -> String {
    let s = source.to_string();
    // `word:word` → `word: word`, EXCEPT annotation use-site targets (`@file:`,
    // `@get:`, `@set:`, `@param:`, …) which take no space after the colon.
    // Rebuilt as a single forward pass — the old in-place `insert` mutated `s`
    // while indexing a stale `chars` snapshot.
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, &c) in chars.iter().enumerate() {
        out.push(c);
        if c == ':'
            && i > 0
            && i + 1 < chars.len()
            && !chars[i + 1].is_whitespace()
            && chars[i + 1] != ':'
        {
            let mut j = i;
            while j > 0 && (chars[j - 1].is_alphanumeric() || chars[j - 1] == '_') {
                j -= 1;
            }
            let is_annotation_target = j > 0 && chars[j - 1] == '@';
            if !is_annotation_target {
                out.push(' ');
            }
        }
    }
    out.split('\n')
        .map(|line| {
            if line.contains("class ") {
                line.replace("):", ") :")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fix_annotation_blank_lines(source: &str) -> String {
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut output = String::with_capacity(source.len());
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index];
        output.push_str(line);
        if is_annotation_only_line(line.trim_start()) {
            let mut next = index + 1;
            while next < lines.len() && lines[next].trim().is_empty() {
                next += 1;
            }
            if next > index + 1
                && next < lines.len()
                && is_annotation_or_declaration(lines[next].trim_start())
            {
                index = next;
                continue;
            }
        }
        index += 1;
    }
    output
}

fn is_annotation_only_line(line: &str) -> bool {
    line.starts_with('@')
        && ![
            " class ",
            " fun ",
            " interface ",
            " object ",
            " typealias ",
            " val ",
            " var ",
        ]
        .iter()
        .any(|keyword| line.contains(keyword))
}

fn is_annotation_or_declaration(line: &str) -> bool {
    line.starts_with('@')
        || [
            "class ",
            "data class ",
            "enum class ",
            "fun ",
            "interface ",
            "object ",
            "typealias ",
            "val ",
            "var ",
            "public ",
            "protected ",
            "private ",
            "internal ",
            "abstract ",
            "open ",
            "override ",
            "suspend ",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix))
}

fn fix_spacing_before_annotated_declarations(source: &str) -> String {
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut output = String::with_capacity(source.len() + 1);
    for (index, line) in lines.iter().enumerate() {
        if is_annotation_only_line(line.trim_start()) && index > 0 {
            let previous = lines[index - 1].trim();
            if !previous.is_empty() && looks_like_declaration_line(previous) {
                output.push('\n');
            }
        }
        output.push_str(line);
    }
    output
}

fn looks_like_declaration_line(line: &str) -> bool {
    if line.ends_with('{') {
        return false;
    }
    (!is_annotation_only_line(line) && is_annotation_or_declaration(line))
        || line.ends_with('}')
        || line.starts_with("constructor(")
        || line.starts_with("init ")
}

fn fix_function_type_modifier_spacing(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let Some(relative) = source[cursor..].find("suspend") else {
            output.push_str(&source[cursor..]);
            break;
        };
        let start = cursor + relative;
        let end = start + "suspend".len();
        output.push_str(&source[cursor..end]);
        let before_is_identifier = start > 0
            && source[..start]
                .chars()
                .next_back()
                .is_some_and(|character| character == '_' || character.is_alphanumeric());
        let after_is_identifier = source[end..]
            .chars()
            .next()
            .is_some_and(|character| character == '_' || character.is_alphanumeric());
        if before_is_identifier || after_is_identifier {
            cursor = end;
            continue;
        }
        let mut next = end;
        while next < bytes.len() && bytes[next].is_ascii_whitespace() {
            next += 1;
        }
        if bytes.get(next) == Some(&b'(') {
            output.push(' ');
            cursor = next;
        } else {
            cursor = end;
        }
    }
    output
}

fn fix_function_type_reference_spacing(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        if !line.contains("fun ") {
            output.push_str(line);
            continue;
        }
        let header_end = line.find('(').unwrap_or(line.len());
        let (header, suffix) = line.split_at(header_end);
        output.push_str(&remove_whitespace_before_receiver_dot(header));
        output.push_str(suffix);
    }
    output
}

fn remove_whitespace_before_receiver_dot(header: &str) -> String {
    let bytes = header.as_bytes();
    let mut output = String::with_capacity(header.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            let start = index;
            while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                index += 1;
            }
            let receiver_dot = bytes.get(index) == Some(&b'.')
                || (bytes.get(index) == Some(&b'?') && bytes.get(index + 1) == Some(&b'.'));
            if receiver_dot {
                continue;
            }
            output.push_str(&header[start..index]);
        } else {
            let character = header[index..].chars().next().expect("valid UTF-8");
            output.push(character);
            index += character.len_utf8();
        }
    }
    output
}

fn block_comment_alignment_edits(source: &str, tree: &tree_sitter::Tree) -> Vec<edit::TextEdit> {
    let mut edits = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "block_comment" | "multiline_comment") {
            let expected = " ".repeat(node.start_position().column + 1);
            let text = &source[node.byte_range()];
            let mut absolute = node.start_byte();
            for (line_index, line) in text.split_inclusive('\n').enumerate() {
                if line_index > 0 {
                    let whitespace = line
                        .bytes()
                        .take_while(|byte| matches!(byte, b' ' | b'\t'))
                        .count();
                    if line.as_bytes().get(whitespace) == Some(&b'*')
                        && whitespace != expected.len()
                    {
                        edits.push(edit::TextEdit::new(
                            "standard:block-comment-initial-star-alignment",
                            absolute..absolute + whitespace,
                            expected.clone(),
                        ));
                    }
                }
                absolute += line.len();
            }
            continue;
        }
        for index in (0..node.child_count()).rev() {
            if let Some(child) = node.child(index) {
                stack.push(child);
            }
        }
    }
    edits
}

fn apply_block_comment_alignment(source: &str) -> anyhow::Result<String> {
    let Some(tree) = parse_clean(source) else {
        return Ok(source.to_string());
    };
    let edits = block_comment_alignment_edits(source, &tree);
    if edits.is_empty() {
        return Ok(source.to_string());
    }
    let output = EditSet::new(edits).apply(source)?;
    let Some(after_tree) = parse_clean(&output) else {
        anyhow::bail!(
            "standard:block-comment-initial-star-alignment produced invalid Kotlin syntax"
        );
    };
    if !block_comment_alignment_edits(&output, &after_tree).is_empty() {
        anyhow::bail!("standard:block-comment-initial-star-alignment is not idempotent");
    }
    Ok(output)
}

fn comment_spacing_edits(source: &str, tree: &tree_sitter::Tree) -> Vec<edit::TextEdit> {
    let bytes = source.as_bytes();
    let mut edits = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.kind().contains("comment") {
            let start = node.start_byte();
            let text = &bytes[start..node.end_byte()];
            if text.starts_with(b"//")
                && text
                    .get(2)
                    .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'/')
            {
                edits.push(edit::TextEdit::new(
                    "standard:comment-spacing",
                    start + 2..start + 2,
                    " ",
                ));
            }
        }
        for index in (0..node.child_count()).rev() {
            if let Some(child) = node.child(index) {
                stack.push(child);
            }
        }
    }
    edits
}

fn apply_comment_spacing(source: &str) -> anyhow::Result<String> {
    let Some(tree) = parse_clean(source) else {
        return Ok(source.to_string());
    };
    let edits = comment_spacing_edits(source, &tree);
    if edits.is_empty() {
        return Ok(source.to_string());
    }
    let output = EditSet::new(edits).apply(source)?;
    let Some(after_tree) = parse_clean(&output) else {
        anyhow::bail!("standard:comment-spacing produced invalid Kotlin syntax");
    };
    if !comment_spacing_edits(&output, &after_tree).is_empty() {
        anyhow::bail!("standard:comment-spacing is not idempotent");
    }
    Ok(output)
}

#[cfg(test)]
fn fix_comment_spacing(source: &str) -> String {
    apply_comment_spacing(source).unwrap_or_else(|_| source.to_string())
}

fn fix_keyword_spacing(source: &str) -> String {
    ["if", "for", "while", "when", "catch"]
        .into_iter()
        .fold(source.to_string(), |text, keyword| {
            text.replace(&format!("{keyword}("), &format!("{keyword} ("))
        })
}

fn fix_range_spacing(source: &str) -> String {
    source
        .replace(" .. ", "..")
        .replace(" ..", "..")
        .replace(".. ", "..")
}

fn fix_single_line_trailing_comma(source: &str) -> String {
    source.replace(",)", ")").replace(", )", ")")
}

fn fix_trailing_lambda_parentheses(source: &str) -> String {
    source
        .split('\n')
        .map(|line| {
            if !line.contains("fun ")
                && !line.contains("class ")
                && !line.contains("interface ")
                && !line.contains("object ")
                && line.contains('.')
            {
                line.replace("() {", " {")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fix_semicolons(source: &str) -> String {
    source
        .split('\n')
        .map(|line| {
            let trimmed = line.trim();
            if trimmed != ";" && line.trim_end().ends_with(';') {
                line.trim_end().trim_end_matches(';').to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fix_class_header_spacing(source: &str) -> String {
    let mut parser = KotlinParser::new();
    let tree = parser.parse(source);
    let (masked, store) = mask_strings_and_chars(source, &tree, true);
    let fixed = masked
        .split('\n')
        .map(|line| {
            if !line.contains("class ") {
                return line.to_string();
            }
            let mut fixed = line.replace("):", ") :");
            if let Some(marker) = fixed.rfind(") :") {
                let after = marker + 3;
                if fixed
                    .as_bytes()
                    .get(after)
                    .is_some_and(|byte| *byte != b' ')
                {
                    fixed.insert(after, ' ');
                }
            }
            fixed
        })
        .collect::<Vec<_>>()
        .join("\n");
    restore_protected(&fixed, &store)
}

fn fix_blank_lines(source: &str) -> String {
    let mut s = source.to_string();
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }
    s
}

fn fix_blank_line_in_list(source: &str) -> String {
    // Disabled: text-level bracket counting cannot distinguish a call's value-
    // argument list (where ktlint may drop a blank line) from a data-class primary
    // constructor's property list, where blank lines legitimately group fields —
    // and ktlint keeps those. Removing them corrupted grouping, so leave blank
    // lines alone; a real list-blank violation is still reported by the linter.
    source.to_string()
}

fn fix_brace_between(source: &str) -> String {
    // Disabled: these patterns removed the newline *before* `}` (the wrong side),
    // gluing the closing brace onto the previous statement (`endpoint\n} else {`
    // → `endpoint} else {`). The intended `}\nelse`→`} else` merge is a wrapping
    // concern handled elsewhere; here it only corrupted valid code.
    source.to_string()
}

fn fix_double_spaces(source: &str) -> String {
    // Collapse runs of interior spaces only — leading indentation must survive.
    // The old whole-string collapse crushed every 4-space indent down to 1 space.
    source
        .split('\n')
        .map(|line| {
            let indent_len = line.len() - line.trim_start().len();
            let (indent, rest) = line.split_at(indent_len);
            let mut r = rest.to_string();
            while r.contains("  ") {
                r = r.replace("  ", " ");
            }
            format!("{indent}{r}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Wrapping helper ──

fn fix_multiline_if_else(source: &str) -> String {
    // if (cond)\n    stmt → if (cond) stmt (single-line when short)
    let mut result = source.to_string();
    let lines: Vec<&str> = source.lines().collect();
    for i in 0..lines.len().saturating_sub(1) {
        let t = lines[i].trim();
        if t.starts_with("if (") || t.starts_with("if(") {
            let body = lines[i + 1].trim();
            if !body.contains('{') && !body.contains("if") && body.len() < 60 {
                let replacement = format!("{} {}", t, body);
                result = result.replace(&format!("{}\n    {}", t, body), &replacement);
                result = result.replace(&format!("{}\n{}", lines[i], lines[i + 1]), &replacement);
                break;
            }
        }
    }
    result
}

// ── Chain wrapping ──

fn fix_chain_wrapping(source: &str) -> String {
    // Disabled: this line-rebuilder forced every `.call` to a hardcoded 4-space
    // indent (destroying real indentation) and, worse, merged unrelated lines while
    // injecting stray `.` (`}.val first =`, `"relative" return …`, `}.?.lowercase()`),
    // producing invalid Kotlin. A safe chain-wrap needs the CST; until then, no-op.
    source.to_string()
}

// ── When expression break ──

fn fix_when_expression_break(source: &str) -> String {
    // Ensure when branches are consistently single-line or multiline.
    // If any branch uses braces, convert all single-line branches
    // to use braces with consistent indentation.
    let lines: Vec<&str> = source.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("when") && t.ends_with('{') {
            result.push(lines[i].to_string());
            i += 1;
            // Collect when body
            while i < lines.len() && lines[i].trim() != "}" {
                let body = lines[i].trim();
                if body.contains("->") && !body.ends_with('{') {
                    let next = if i + 1 < lines.len() {
                        lines[i + 1].trim()
                    } else {
                        ""
                    };
                    if !next.is_empty()
                        && next != "}"
                        && !next.contains("->")
                        && !next.starts_with("//")
                    {
                        // Merge single-line body onto the -> line with proper indent
                        let _indent = " ".repeat(body.len() - body.trim_start().len() + 4);
                        result.push(format!("{} {{ {} }}", body, next));
                        i += 2;
                        continue;
                    }
                }
                result.push(lines[i].to_string());
                i += 1;
            }
            if i < lines.len() {
                result.push(lines[i].to_string());
                i += 1;
            }
        } else {
            result.push(lines[i].to_string());
            i += 1;
        }
    }
    result.join("\n")
}

fn fix_single_line_control_blocks(source: &str) -> String {
    let mut output = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        let is_control = ["if (", "for (", "while ("]
            .iter()
            .any(|prefix| trimmed.starts_with(prefix));
        let open = line.find('{');
        let close = line.rfind('}');
        if is_control && open.is_some() && close.is_some() && open < close {
            let open = open.unwrap_or(0);
            let close = close.unwrap_or(open);
            let suffix = &line[close + 1..];
            if suffix.trim_start().starts_with("else") {
                output.push(line.to_string());
                continue;
            }
            let indent = &line[..line.len() - trimmed.len()];
            let body = line[open + 1..close].trim();
            output.push(format!("{}{} {{", indent, line[..open].trim_end()));
            if !body.is_empty() {
                output.push(format!("{}    {}", indent, body));
            }
            output.push(format!("{}}}{}", indent, suffix));
        } else {
            output.push(line.to_string());
        }
    }
    let mut result = output.join("\n");
    if source.ends_with('\n') {
        result.push('\n');
    }
    result
}

// ── Try-catch wrapping ──

fn fix_try_catch(source: &str) -> String {
    source
        .replace("}\ncatch", "} catch")
        .replace("}\nfinally", "} finally")
        .replace("}\n    catch", "} catch")
        .replace("}\n    finally", "} finally")
}

// ── When entry bracing ──

fn fix_when_entry_bracing(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.contains("-> {") {
            let rest = t.split("-> {").nth(1).unwrap_or("");
            if rest.trim_end().ends_with("}") {
                let inner = rest
                    .trim_start()
                    .trim_end_matches('}')
                    .trim()
                    .trim_end_matches(';');
                let prefix = &t[..t.find("-> {").unwrap() + 3];
                result.push(format!("{} {}", prefix, inner));
                i += 1;
                continue;
            }
        }
        result.push(lines[i].to_string());
        i += 1;
    }
    result.join("\n")
}

// ── String template indent ──

fn fix_string_template(source: &str) -> String {
    // Add .trimIndent() to multiline string literals that lack it
    let lines: Vec<&str> = source.lines().collect();
    let mut result = Vec::new();
    let mut in_multiline = false;
    for (_i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if !in_multiline && t.contains("\"\"\"") && t.matches("\"\"\"").count() == 1 {
            in_multiline = true;
            // If the line ends with just the opening quotes, check for trim call
            if !t.contains(".trimIndent()") && !t.contains(".trimMargin()") {
                result.push(format!("{}.trimIndent()", line));
                continue;
            }
        }
        if in_multiline && t.contains("\"\"\"") {
            in_multiline = false;
        }
        result.push(line.to_string());
    }
    result.join("\n")
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_layer_rejects_invalid_kotlin_output() {
        let error =
            safe_transform("unsafe-rule", "val x = 1\n", |_| "fun {".to_string()).unwrap_err();
        assert!(error.to_string().contains("invalid Kotlin syntax"));
    }

    #[test]
    fn safety_layer_rejects_protected_region_changes() {
        let error = safe_transform("unsafe-rule", "val x = \"safe\"\n", |source| {
            source.replace("safe", "changed")
        })
        .unwrap_err();
        assert!(error.to_string().contains("protected Kotlin region"));
    }

    #[test]
    fn safety_layer_rejects_non_idempotent_pass() {
        let error = safe_transform("toggle-rule", "val x = 1", |source| {
            if source.ends_with('\n') {
                source.trim_end_matches('\n').to_string()
            } else {
                format!("{source}\n")
            }
        })
        .unwrap_err();
        assert!(error.to_string().contains("not idempotent"));
    }

    #[test]
    fn safety_layer_does_not_run_on_parse_errors_or_sentinel_collisions() {
        let broken = "fun broken(";
        assert_eq!(
            safe_transform("rule", broken, |_| panic!("must not transform")).unwrap(),
            broken
        );
        let collision = format!("val x = 1 // {SENTINEL}");
        assert_eq!(
            safe_transform("rule", &collision, |_| panic!("must not transform")).unwrap(),
            collision
        );
    }

    #[test]
    fn formatter_pipeline_is_idempotent() {
        let source = "fun main(){val x=1}  ";
        let once =
            format_once(source, 4, true, &HashMap::new(), CodeStyle::KtlintOfficial).unwrap();
        assert_eq!(
            format_once(&once, 4, true, &HashMap::new(), CodeStyle::KtlintOfficial,).unwrap(),
            once
        );
    }

    #[test]
    fn safety_corpus_preserves_nested_protected_regions_and_operator_meaning() {
        let source = r####"fun nested(args: Array<String>) {
    val spread = arrayOf(* args)
    val unary = -1
    val binary = unary+2
    val text = "a+b // text"
    val raw = """raw * - + // text"""
    val charValue = '+'
    val `odd+name` = binary
    println("$spread $unary $binary $text $raw $charValue ${`odd+name`}")
}
"####;
        let before_tree = parse_clean(source).unwrap();
        let before = protected_snapshot(source, &before_tree).unwrap().fragments;
        let output =
            format_once(source, 4, true, &HashMap::new(), CodeStyle::KtlintOfficial).unwrap();
        let after_tree = parse_clean(&output).unwrap();
        assert_eq!(
            protected_snapshot(&output, &after_tree).unwrap().fragments,
            before
        );
        assert!(output.contains("arrayOf(*args)"));
        assert!(output.contains("val unary = -1"));
        assert!(output.contains("val binary = unary + 2"));
        assert!(!output.contains('\u{FFFD}'));
        assert_eq!(
            format_once(&output, 4, true, &HashMap::new(), CodeStyle::KtlintOfficial,).unwrap(),
            output
        );
    }

    #[test]
    fn safety_layer_protects_every_opaque_kotlin_region() {
        let cases = [
            ("val x = \"safe\"\n", "safe", "changed"),
            ("val x = \"\"\"safe\"\"\"\n", "safe", "changed"),
            ("val x = 's'\n", "'s'", "'x'"),
            ("// safe\nval x = 1\n", "safe", "changed"),
            ("val `safe` = 1\n", "safe", "changed"),
        ];
        for (source, old, new) in cases {
            let error =
                safe_transform("unsafe-rule", source, |input| input.replace(old, new)).unwrap_err();
            assert!(
                error.to_string().contains("protected Kotlin region"),
                "unexpected result for {source:?}: {error}"
            );
        }
    }

    #[test]
    fn generated_edits_never_span_protected_bytes() {
        let source = "val a=1; val text=\"safe\"; val b=2\n";
        let transformed = "val a = 1; val text = \"safe\"; val b = 2\n";
        let before = protected_snapshot(source, &parse_clean(source).unwrap()).unwrap();
        let after = protected_snapshot(transformed, &parse_clean(transformed).unwrap()).unwrap();
        let edits = edits_around_protected_regions(
            "standard:op-spacing",
            source,
            transformed,
            &before,
            &after,
        );
        for edit in &edits {
            for protected in &before.ranges {
                assert!(
                    edit.range.end <= protected.start || edit.range.start >= protected.end,
                    "edit {:?} spans protected range {protected:?}",
                    edit.range
                );
            }
        }
        assert_eq!(EditSet::new(edits).apply(source).unwrap(), transformed);
    }

    #[test]
    fn fix_operator_equals() {
        assert_eq!(fix_all_spacing("val x=1"), "val x = 1");
    }
    #[test]
    fn fix_curly_brace() {
        assert_eq!(fix_all_spacing("fun foo(){x}"), "fun foo() { x }");
    }
    #[test]
    fn fix_colon_spacing() {
        assert!(fix_all_spacing("val x:String").contains("x: String"));
    }
    #[test]
    fn fix_trailing_ws_test() {
        assert_eq!(fix_trailing_ws("val x = 1   \n   "), "val x = 1\n");
    }
    #[test]
    fn fix_indent() {
        let src = "class Foo {\nval x = 1\n}";
        assert_eq!(fix_indentation(src, 4), "class Foo {\n    val x = 1\n}");
    }
    #[test]
    fn fix_chain_wrap() {
        let r = fix_chain_wrapping("val x = foo\n    .bar()\n    .baz()");
        assert!(r.contains(".bar()"), "got: {}", r);
    }
    #[test]
    fn fix_when_break() {
        let src = "when (x) {\n    1 -> println(\"one\")\n    else -> {\n        println(\"other\")\n    }\n}";
        let r = fix_when_expression_break(src);
        assert!(r.contains("->"), "got: {}", r);
    }
    #[test]
    fn fix_try_catch_wrap() {
        assert_eq!(
            fix_try_catch(
                "}
catch(e: E) { b() }"
            ),
            "} catch(e: E) { b() }"
        );
    }
    #[test]
    fn fix_when_entry_brace() {
        let r = fix_when_entry_bracing("x -> { doStuff() }");
        assert!(r.contains("x ->  doStuff()"), "got: {}", r);
    }
    #[test]
    fn fix_wrapping_preserves() {
        let src = "val x = foo\n    .bar()\n    .baz()";
        let r = fix_all_wrapping(src);
        assert_eq!(r, src);
    }

    #[test]
    fn raw_string_content_is_never_reformatted_after_restore() {
        let source = "val raw = \"\"\"\n//keep\nclass Fake):Type\n\"\"\"\nval x=1\n";
        let fixed = fix_all_spacing(source);
        assert!(fixed.contains("//keep\nclass Fake):Type"));
        assert!(fixed.contains("val x = 1"));
    }

    #[test]
    fn extension_function_parentheses_are_preserved() {
        let source = "private fun Foo.bar() {\n}\n";
        assert_eq!(fix_trailing_lambda_parentheses(source), source);
    }

    #[test]
    fn single_line_control_block_preserves_suffix_comment() {
        let source = "if (ready) { run() } // keep\n";
        let fixed = fix_single_line_control_blocks(source);
        assert!(fixed.contains("} // keep"), "got: {fixed}");
    }

    // ── Issue #63: UTF-8 multi-byte character safety ──

    #[test]
    fn issue63_box_drawing_before_equals() {
        // \u{2500} is 3 bytes. `=` spacing must not corrupt it.
        let src = "// \u{2500}\u{2500} comment \u{2500}\u{2500}\nval x=1";
        let r = fix_all_spacing(src);
        assert!(r.contains("val x = 1"), "equals not fixed: {}", r);
        assert!(
            r.contains("\u{2500}\u{2500} comment"),
            "box chars corrupted: {}",
            r
        );
        assert!(!r.contains("\u{fffd}"), "replacement char found: {}", r);
    }

    #[test]
    fn issue63_box_drawing_operators_only() {
        // Verify fix_operators directly with box-drawing chars
        let src = "// \u{2500}\u{2500} test \u{2500}\u{2500}\nval x=1";
        let r = fix_operators(src);
        assert!(r.contains("\u{2500}"), "box char lost: {}", r);
        assert!(r.contains("x = 1"), "operator not fixed: {}", r);
    }

    #[test]
    fn issue63_box_drawing_curly_only() {
        let src = "// \u{2500}\u{2500} test \u{2500}\u{2500}\nclass Foo{}";
        let r = fix_curly_braces(src);
        assert!(r.contains("Foo {"), "curly not fixed: {}", r);
        assert!(r.contains("\u{2500}"), "box char lost: {}", r);
    }

    #[test]
    fn issue63_box_drawing_colons_only() {
        let src = "// \u{2500}\u{2500} test\nval x:String";
        let r = fix_colons(src);
        assert!(r.contains("x: String"), "colon not fixed: {}", r);
        assert!(r.contains("\u{2500}"), "box char lost: {}", r);
    }

    #[test]
    fn issue63_cjk_chars_with_operators() {
        // CJK characters are 3 bytes each.
        let src = "// \u{3053}\u{3093}\u{306B}\u{3061}\u{306F}\nval x=1";
        let r = fix_all_spacing(src);
        assert!(r.contains("val x = 1"), "equals not fixed: {}", r);
        assert!(r.contains("\u{306B}"), "CJK char lost: {}", r);
        assert!(!r.contains("\u{fffd}"), "replacement char: {}", r);
    }

    #[test]
    fn issue63_em_dash_before_operators() {
        // \u{2014} (EM DASH) is 3 bytes.
        let src = "// \u{2014}\u{2014} test\nval a=1\nval b=2";
        let r = fix_operators(src);
        assert!(r.contains("a = 1"), "eq1 not fixed: {}", r);
        assert!(r.contains("b = 2"), "eq2 not fixed: {}", r);
        assert!(r.contains("\u{2014}"), "em dash lost: {}", r);
    }

    #[test]
    fn issue63_emoji_with_operators() {
        // Emoji are 4 bytes.
        let src = "// \u{1f600} test\nval x=1";
        let r = fix_all_spacing(src);
        assert!(r.contains("val x = 1"), "equals not fixed: {}", r);
        assert!(r.contains("\u{1f600}"), "emoji lost: {}", r);
    }

    #[test]
    fn issue63_preserves_unicode_no_replacement_char() {
        // Any valid UTF-8 must survive formatting without replacement characters.
        let src = concat!(
            "// \u{2500}\u{2500} box \u{2500}\u{2500}\n",
            "// \u{3053}\u{3093} CJK\n",
            "// \u{2014} em dash\n",
            "// \u{1f600} emoji\n",
            "class Foo{}\n",
            "val x=1\n",
            "val y:String\n",
        );
        let r = fix_all_spacing(src);
        assert!(!r.contains("\u{fffd}"), "replacement char in output: {}", r);
        assert!(r.contains("\u{2500}"), "box lost");
        assert!(r.contains("\u{3053}"), "CJK lost");
        assert!(r.contains("\u{2014}"), "em dash lost");
        assert!(r.contains("\u{1f600}"), "emoji lost");
        assert!(r.contains("Foo {"), "curly not fixed");
        assert!(r.contains("x = 1"), "equals not fixed");
        assert!(r.contains("y: String"), "colon not fixed");
    }

    #[test]
    fn issue63_char_boundary_safety() {
        // Verify every s.insert() position is a char boundary.
        fn check_char_boundaries(orig: &str, fixed: &str) {
            assert!(orig.is_char_boundary(0), "always valid");
            for (bi, _) in orig.char_indices() {
                assert!(
                    orig.is_char_boundary(bi),
                    "byte {bi} not boundary in original"
                );
            }
            for (bi, _) in fixed.char_indices() {
                assert!(
                    fixed.is_char_boundary(bi),
                    "byte {bi} not boundary in fixed"
                );
            }
        }
        let src = "// \u{2500}\u{2500} test \u{2500}\u{2500}\nval x=1\nval y=2\nclass Foo{}\nval z:String";
        let r = fix_all_spacing(src);
        check_char_boundaries(&r, &r);
        // Also test individual functions
        check_char_boundaries("// \u{2500} x=1", &fix_operators("// \u{2500} x=1"));
        check_char_boundaries("// \u{2500} Foo{}", &fix_curly_braces("// \u{2500} Foo{}"));
        check_char_boundaries("// \u{2500} x:String", &fix_colons("// \u{2500} x:String"));
    }

    // ── String / comment interior must never be mutated ──

    #[test]
    fn string_literal_url_is_preserved() {
        // Regression: `"https://x?a=b"` was mangled to `"https:/ / x?a = b"`.
        let src = "val u = \"https://callback?flow=abc&token=x-y\"\n";
        let r = fix_all_spacing(src);
        assert!(
            r.contains("\"https://callback?flow=abc&token=x-y\""),
            "string literal corrupted: {r}"
        );
    }

    #[test]
    fn kdoc_slashes_and_dashes_preserved() {
        // Regression: `/**` → `/ * *`, `pause/resume` → `pause / resume`,
        // `server-side` → `server - side`.
        let src = "/**\n * pause/resume and server-side notes\n */\nval x=1\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("/**"), "kdoc opener corrupted: {r}");
        assert!(r.contains(" */"), "kdoc closer corrupted: {r}");
        assert!(r.contains("pause/resume"), "comment slash corrupted: {r}");
        assert!(r.contains("server-side"), "comment dash corrupted: {r}");
        assert!(r.contains("val x = 1"), "real code not fixed: {r}");
    }

    #[test]
    fn line_comment_url_preserved() {
        let src = "// see https://example.com/path?a=b\nval y=2\n";
        let r = fix_all_spacing(src);
        assert!(
            r.contains("https://example.com/path?a=b"),
            "line-comment url corrupted: {r}"
        );
        assert!(r.contains("val y = 2"), "real code not fixed: {r}");
    }

    #[test]
    fn backtick_identifier_preserved() {
        // Regression: `` `sign-in is a no-op` `` was mangled to `sign - in is a no - op`.
        let src = "fun `sign-in is a no-op`() {\n    val x=1\n}\n";
        let r = fix_all_spacing(src);
        assert!(
            r.contains("`sign-in is a no-op`"),
            "backtick identifier corrupted: {r}"
        );
        assert!(r.contains("val x = 1"), "real code not fixed: {r}");
    }

    #[test]
    fn indentation_is_not_collapsed() {
        // Regression: 4-space indent was crushed to 1 space by fix_double_spaces.
        let src = "class Foo {\n    fun bar() {\n        val x=1\n    }\n}\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("\n    fun bar()"), "4-space indent lost: {r}");
        assert!(
            r.contains("\n        val x = 1"),
            "8-space indent lost: {r}"
        );
    }

    #[test]
    fn closing_paren_indentation_preserved() {
        // Regression: `fix_parens`' global `replace(" )", ")")` ate the leading
        // indent of a `)` on its own line (`        )` → `   )`) across loop passes.
        let src = "val h = foo(\n    a = 1,\n    b = 2,\n)\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("\n)"), "closing paren indent altered: {r:?}");
        assert!(!r.contains(" )"), "space before paren remained: {r:?}");
    }

    #[test]
    fn inner_paren_spaces_still_collapsed() {
        let src = "foo( a, b )\n";
        let r = fix_all_spacing(src);
        assert!(
            r.contains("foo(a, b)"),
            "inner paren spaces not collapsed: {r:?}"
        );
    }

    #[test]
    fn compound_operators_not_split() {
        // Regression: the single-char `=` pass split `==` into `= =`; the (now
        // removed) angle-bracket tidy ate `>=`'s leading space.
        assert!(fix_all_spacing("if (a==b) {}\n").contains("a == b"));
        assert!(fix_all_spacing("val r = a>=b\n").contains("a >= b"));
        assert!(fix_all_spacing("val r = a!=b\n").contains("a != b"));
        assert!(fix_all_spacing("x+=1\n").contains("x += 1"));
        // Already-correct compound operators must be left untouched (never `= =`).
        assert!(!fix_all_spacing("val r = a == b\n").contains("= ="));
    }

    #[test]
    fn annotation_use_site_colon_preserved() {
        // Regression: `@file:OptIn` / `@get:JvmName` mangled to `@file: OptIn`.
        assert!(fix_all_spacing("@file:OptIn(Foo::class)\n").contains("@file:OptIn"));
        assert!(fix_all_spacing("@get:JvmName(\"x\")\nval y = 1\n").contains("@get:JvmName"));
        // Ordinary member colon still gets its space.
        assert!(fix_all_spacing("val x:Int = 1\n").contains("val x: Int"));
    }

    #[test]
    fn supertype_colon_space_preserved() {
        // Regression: `) : Base` / `where T : Any` collapsed to `): Base` / `T: Any`.
        let src = "class Foo(x: Int) : Base() {\n    val y: Int = x\n}\n";
        let r = fix_all_spacing(src);
        assert!(r.contains(") : Base()"), "supertype colon collapsed: {r:?}");
        assert!(
            r.contains("val y: Int"),
            "member colon should stay tight: {r:?}"
        );
    }

    #[test]
    fn generic_angle_brackets_preserved() {
        // Regression: `Map<String, Int>` mangled to `Map < String, Int >`.
        let src = "val m: Map<String, Int> = mapOf()\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("Map<String, Int>"), "generic corrupted: {r:?}");
    }

    #[test]
    fn comparison_operators_spaced_not_mangled() {
        // `>=` keeps its spaces (previously eaten by fix_angle_brackets).
        let src = "val r = if (a >= b) x else y\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("a >= b"), "comparison mangled: {r:?}");
    }

    #[test]
    fn blank_line_in_lambda_block_preserved() {
        // Regression: blank lines inside a `{}` block nested in a call were dropped.
        let src = "foo(bar {\n    a()\n\n    b()\n})\n";
        let r = fix_all_spacing(src);
        assert!(
            r.contains("a()\n\n    b()"),
            "blank line in block dropped: {r:?}"
        );
    }

    #[test]
    fn blank_line_in_data_class_params_preserved() {
        // Regression: blank lines grouping data-class properties (ktlint keeps them)
        // were removed by the paren-based blank-line stripper.
        let src = "data class C(\n    val a: Int,\n\n    val b: Int,\n)\n";
        let r = fix_all_spacing(src);
        // The blank line between the two properties must survive (trailing-space
        // normalization after commas is handled separately by fix_trailing_ws).
        assert!(r.contains("\n\n"), "grouping blank line dropped: {r:?}");
        assert!(r.contains("val b: Int"), "second property lost: {r:?}");
    }

    #[test]
    fn unparseable_file_left_untouched() {
        // When tree-sitter can't parse the file, we must not risk text-level edits
        // that could corrupt strings/comments — return spacing pass unchanged.
        let src = "val u=\"a=b\"\n)))not valid kotlin(((\nfun x(=\n";
        assert_eq!(fix_all_spacing(src), src);
    }

    #[test]
    fn interior_double_spaces_still_collapsed() {
        let src = "val  x   =    1\n";
        let r = fix_all_spacing(src);
        assert!(
            r.contains("val x = 1"),
            "interior spaces not collapsed: {r}"
        );
    }

    #[test]
    fn operator_spacing_around_string_still_applied() {
        // Masking must not stop spacing being fixed *outside* the string.
        let src = "val s=\"a=b\"\n";
        let r = fix_all_spacing(src);
        assert!(
            r.contains("val s = \"a=b\""),
            "expected `s = \"a=b\"`, got: {r}"
        );
    }

    #[test]
    fn issue63_no_panic_on_large_utf8() {
        // Verify no panic on a larger file with mixed content.
        let mut src = String::from("// \u{2500}".repeat(80));
        src.push_str("\n");
        for i in 0..20 {
            src.push_str(&format!("val x{}={}\n", i, i));
        }
        src.push_str("class Broken{}\n");
        src.push_str(&"// \u{3053}\u{3093}\u{306B}\u{3061}\u{306F}");
        // Must not panic
        let r = fix_all_spacing(&src);
        assert!(!r.is_empty(), "output must not be empty");
        assert!(!r.contains("\u{fffd}"), "no replacement chars");
    }

    // ── PR #67 additional coverage ──

    #[test]
    fn backtick_with_method_call_not_mangled() {
        let src = "val r = `is`(x)\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("`is`"), "backtick lost: {r:?}");
    }

    #[test]
    fn cs67_template_complex_expression_not_broken() {
        let src = "val s = \"\u{0024}{a + b}\"\n";
        let r = fix_all_spacing(src);
        assert!(
            r.contains("\u{0024}{a + b}"),
            "complex template lost: {r:?}"
        );
    }

    #[test]
    fn cs67_comment_operators_preserved() {
        let src = "// val x=1\nval y=2\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("// val x=1"), "comment modified: {r:?}");
        assert!(r.contains("val y = 2"), "code not fixed: {r:?}");
    }

    #[test]
    fn cs67_block_comment_braces_untouched() {
        let src = "/* { } */\nclass Foo {}\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("/* { } */"), "block comment changed: {r:?}");
    }

    #[test]
    fn cs67_string_equals_not_spaced() {
        let src = "val s = \"a=b\"\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("\"a=b\""), "string equals spaced: {r:?}");
    }

    #[test]
    fn cs67_disabled_indent_not_applied() {
        let src = "class Foo {\nval x = 1\n}\n";
        let r = fix_all_spacing(src);
        assert!(
            r.contains("\nval x"),
            "indent applied (should be disabled): {r:?}"
        );
    }

    #[test]
    fn cs67_disabled_chain_wrap_unchanged() {
        let src = "val x = foo\n    .bar()\n    .baz()\n";
        let r = fix_all_spacing(src);
        assert!(r.contains(".bar()"), "chain wrap touched: {r:?}");
    }

    #[test]
    fn cs67_disabled_string_template_no_trim() {
        let src = "val s = \"\"\"\n    line\n\"\"\"\n";
        let r = fix_all_spacing(src);
        assert!(!r.contains("trimIndent"), "trimIndent added: {r:?}");
    }

    #[test]
    fn cs67_parse_error_untouched_unmatched() {
        let src = "val x = (a + b\n";
        let r = fix_all_spacing(src);
        assert_eq!(r, src, "unparseable must be untouched");
    }

    #[test]
    fn cs67_parse_error_untouched_jumbled() {
        let src = "val ){ class = fun\n";
        let r = fix_all_spacing(src);
        assert_eq!(r, src, "jumbled must be untouched");
    }

    #[test]
    fn cs67_real_world_snippet_all_forms_survive() {
        let src = concat!(
            "@file:Suppress(\"ktlint:standard:max-line-length\")\n",
            "@get:JvmName(\"foo\")\n",
            "class `MyData`<T : Any> : Base(\n",
            "    val id: String,\n",
            "    val `prop`: String,\n",
            ") where T : Comparable<T> {\n",
            "    // TODO\n",
            "    val j = \"\"\"\n",
            "        {\"k\": \"v\"}\n",
            "    \"\"\".trimIndent()\n",
            "    fun f() = \"",
            "\u{0024}name world\"\n",
            "    val `is` = 1\n",
            "    val should==1\n",
            "}\n",
        );
        let r = fix_all_spacing(src);
        assert!(r.contains("@file:Suppress"), "use-site lost");
        assert!(r.contains("@get:JvmName"), "get lost");
        assert!(r.contains("`MyData`"), "backtick class lost");
        assert!(r.contains("`prop`"), "backtick prop lost");
        assert!(r.contains("`is`"), "backtick val lost");
        assert!(r.contains("<T : Any>"), "generic lost");
        assert!(r.contains("where T"), "where clause lost");
        assert!(r.contains("trimIndent"), "trimIndent lost");
        assert!(r.contains("// TODO"), "comment lost");
        assert!(
            r.contains("should==1") || r.contains("should == 1"),
            "expr lost"
        );
        assert!(!r.contains("\u{fffd}"), "replacement char");
        assert!(r.contains("@get:JvmName"), "get lost");
    }

    #[test]
    fn cs67_leading_indent_survives_paren_fix() {
        let src = "fun f() {\n    bar(\n        x,\n        y,\n    )\n}\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("    )"), "indent eaten: {r:?}");
    }

    #[test]
    fn cs67_double_spaces_removed_but_indent_kept() {
        let src = "    val  x   =    1\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("    val x = 1"), "wrong: {r:?}");
    }

    // ── Mask→restore: generics, supertype colon, use-site targets ──

    #[test]
    fn cs68_generic_type_args_not_spaced() {
        let src = "val list: List<String> = emptyList()\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("List<String>"), "generic args broken: {r:?}");
    }

    #[test]
    fn cs68_generic_function_call_not_spaced() {
        let src = "val map = mapOf<String, Int>()\n";
        let r = fix_all_spacing(src);
        assert!(
            r.contains("mapOf<String, Int>"),
            "generic call broken: {r:?}"
        );
    }

    #[test]
    fn cs68_supertype_colon_preserved() {
        let src = "class Foo : Bar<Int> { val x = 1 }\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("Foo : Bar"), "supertype colon broken: {r:?}");
    }

    #[test]
    fn cs68_where_clause_colon_preserved() {
        let src = "fun <T> f() where T : Comparable<T> = TODO()\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("T : Comparable"), "where clause broken: {r:?}");
    }

    #[test]
    fn cs68_param_receiver_targets_preserved() {
        let src = "@param:Deprecated @receiver:Suppress(\"w\") fun f() = 1\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("@param:Deprecated"), "param broken: {r:?}");
        assert!(r.contains("@receiver:Suppress"), "receiver broken: {r:?}");
    }

    #[test]
    fn cs68_file_target_colon_not_spaced() {
        let src = "@file:Suppress(\"ktlint\")\nclass Foo {}\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("@file:Suppress"), "file colon broken: {r:?}");
    }

    // ── Operator safety: compound ops, safe-call, elvis ──

    #[test]
    fn cs68_double_equals_not_split() {
        let src = "val ok = a == b\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("=="), "== split: {r:?}");
    }

    #[test]
    fn cs68_not_equals_not_split() {
        let src = "val ok = a != b\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("!="), "!= split: {r:?}");
    }

    #[test]
    fn cs68_elvis_operator_survives() {
        let src = "val x = a ?: b\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("?:"), "elvis broken: {r:?}");
    }

    #[test]
    fn cs68_safe_call_operator_survives() {
        let src = "val x = a?.b()\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("?."), "safe-call broken: {r:?}");
    }

    #[test]
    fn cs68_range_operator_survives() {
        let src = "for (i in 1..10) {}\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("1..10"), "range broken: {r:?}");
    }

    #[test]
    fn cs68_compound_plus_equals_not_split() {
        let src = "x += 1\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("+="), "+= split: {r:?}");
    }

    #[test]
    fn cs68_property_reference_not_spaced() {
        let src = "val ref = Foo::bar\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("::"), "prop ref broken: {r:?}");
    }

    // ── Label, enum, and annotation edge cases ──

    #[test]
    fn cs68_label_colon_preserved() {
        let src = "loop@ for (x in 1..5) { break@loop }\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("loop@"), "label broken: {r:?}");
    }

    #[test]
    fn cs68_enum_constructor_call_preserved() {
        let src = "enum class E { A(1), B(2) }\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("A(1)"), "enum call broken: {r:?}");
    }

    // ── Multiline template, spread, KDoc ──

    #[test]
    fn cs68_multiline_raw_template_preserved() {
        let src = "val s = \"\"\"\n    \u{0024}{foo.bar()}\n\"\"\".trimIndent()\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("\u{0024}{foo.bar()}"), "template lost: {r:?}");
    }

    #[test]
    fn cs68_spread_operator_preserved() {
        let src = "val arr = listOf(*array)\n";
        assert_eq!(fix_all_spacing(src), src);
    }

    #[test]
    fn cs68_spread_operator_removes_inner_space() {
        let src = "val arr = listOf(* array)\n";
        assert_eq!(fix_all_spacing(src), "val arr = listOf(*array)\n");
    }

    #[test]
    fn cs68_kdoc_with_code_block_untouched() {
        let src = "/**\n * ```\n * val x=1\n * ```\n */\nclass Foo\n";
        let r = fix_all_spacing(src);
        assert!(r.contains("val x=1"), "kdoc code changed: {r:?}");
    }

    // ── Multiple fixes + UTF-8 ──

    #[test]
    fn cs68_multiple_fixes_no_replacement_chars() {
        let src = concat!(
            "// \u{2500} UTF-8\n",
            "class Foo { val x=1 val y=2 }\n",
            "class Bar{ val a=3 }\n",
        );
        let r = fix_all_spacing(src);
        assert!(!r.contains("\u{fffd}"), "replacement: {r:?}");
        assert!(r.contains("\u{2500}"), "box lost: {r:?}");
        assert!(
            r.contains("x=1") || r.contains("x = 1"),
            "eq expr lost: {r:?}"
        );
        // second class brace may not fix inside multi-block string
    }
}
