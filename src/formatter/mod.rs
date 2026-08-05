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
    // Super-type colons are repaired by fix_colons itself (it distinguishes
    // `class Foo(...) : Base` from constructor parameter colons). Do not mask
    // them away here, otherwise fix_colons never sees `object Bar:Base`.
    let _ = node;
    false
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

/// ktlint 1.8 rules that are NOT enabled by default (must be explicitly
/// `= enabled` in .editorconfig). Mirrors config::DEFAULT_DISABLED_RULES.
const DEFAULT_DISABLED_RULES: &[&str] = &["standard:expression-operand-wrapping"];

fn rule_enabled(
    rule_configs: &HashMap<String, RuleConfig>,
    rule_id: &str,
    code_style: CodeStyle,
) -> bool {
    crate::rules::code_style_allows(rule_id, code_style)
        && match rule_configs.get(rule_id) {
            // Explicit .editorconfig wins (enabled or disabled).
            Some(config) => config.enabled,
            // Otherwise ktlint 1.8 default-disabled rules stay off.
            None => !DEFAULT_DISABLED_RULES.contains(&rule_id),
        }
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
        let passes: [(&'static str, fn(&str) -> String); 30] = [
            ("standard:annotation-spacing", fix_annotation_blank_lines),
            ("standard:modifier-list-spacing", fix_annotation_blank_lines),
            (
                "standard:spacing-between-declarations-with-annotations",
                fix_spacing_before_annotated_declarations,
            ),
            (
                "standard:expression-operand-wrapping",
                fix_expression_operand_wrapping,
            ),
            (
                "standard:modifier-list-spacing",
                fix_context_receiver_list_wrapping,
            ),
            (
                "standard:context-receiver-list-wrapping",
                fix_context_receiver_list_wrapping,
            ),
            ("standard:no-empty-class-body", fix_empty_class_body),
            (
                "standard:parameter-list-wrapping",
                fix_parameter_list_wrapping,
            ),
            (
                "standard:function-type-modifier-spacing",
                fix_function_type_modifier_spacing,
            ),
            (
                "standard:function-type-reference-spacing",
                fix_function_type_reference_spacing,
            ),
            ("standard:double-colon-spacing", fix_double_colons),
            ("standard:curly-spacing", fix_curly_braces),
            ("standard:op-spacing", fix_operators),
            ("standard:comma-spacing", fix_commas),
            ("standard:paren-spacing", fix_parens),
            ("standard:spacing-around-angle-brackets", fix_angle_brackets),
            ("standard:colon-spacing", fix_colons),
            ("standard:fun-keyword-spacing", fix_keyword_spacing),
            ("standard:function-return-type-spacing", fix_colons),
            ("standard:dot-spacing", fix_dot_spacing),
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
            (
                "standard:blank-line-between-when-conditions",
                fix_when_conditions_blank_lines,
            ),
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
    max_line_length: usize,
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
    if rule_enabled(rule_configs, "standard:argument-list-wrapping", code_style)
        || rule_enabled(rule_configs, "standard:property-wrapping", code_style)
        || rule_enabled(rule_configs, "standard:function-signature", code_style)
    {
        text = safe_transform("standard:wrapping-fix", &text, |input| {
            fix_wrapping(input, indent_size, max_line_length)
        })?;
    }
    if rule_enabled(rule_configs, "standard:statement-wrapping", code_style) {
        text = safe_transform("standard:statement-wrapping", &text, |input| {
            fix_statement_wrapping(input, indent_size)
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

/// Format an in-memory source string (used by `--stdin --format`). Mirrors
/// `auto_fix`'s whole-file pipeline without touching the filesystem.
pub fn format_source(
    source: &str,
    indent_size: usize,
    insert_final_newline: bool,
    rule_configs: &HashMap<String, RuleConfig>,
    code_style: CodeStyle,
    max_line_length: usize,
) -> anyhow::Result<String> {
    if source.contains(SENTINEL) || parse_clean(source).is_none() {
        return Ok(source.to_string());
    }
    let text = format_once(
        source,
        indent_size,
        insert_final_newline,
        rule_configs,
        code_style,
        max_line_length,
    )?;
    if format_once(
        &text,
        indent_size,
        insert_final_newline,
        rule_configs,
        code_style,
        max_line_length,
    )? != text
    {
        anyhow::bail!("formatter pipeline is not idempotent for stdin input");
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
    max_line_length: usize,
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
            max_line_length,
        )?;
        match format_once(
            &text,
            indent_size,
            insert_final_newline,
            rule_configs,
            code_style,
            max_line_length,
        ) {
            Ok(t) if t != text => {
                anyhow::bail!("formatter pipeline is not idempotent for {file_path}");
            }
            Ok(_) => {}
            Err(e) => {
                anyhow::bail!("formatter pipeline second pass failed for {file_path}: {e}");
            }
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

/// Wrapping auto-fixes mirroring ktlint's autocorrect for the wrapping rules
/// (function-signature body-merge, property-wrapping, argument-list-wrapping).
/// Each pass only touches *single-line overlong* constructs; multiline inputs
/// are left to the indent rule. Edits are applied bottom-up so offsets stay
/// valid. All passes are conservative: they never touch strings/comments (via
/// masked_transform in the caller) and only fire when the line actually
/// exceeds max_line_length.
fn fix_wrapping(source: &str, indent_size: usize, max_line_length: usize) -> String {
    let max_line_length = if max_line_length == 0 {
        120
    } else {
        max_line_length
    };
    let mut text = source.to_string();
    text = fix_function_body_merge(&text, max_line_length);
    text = fix_property_wrapping(&text, indent_size, max_line_length);
    text = fix_argument_list_wrapping(&text, indent_size, max_line_length);
    text
}

/// `fun foo(...): Type =` + newline + body whose first line fits on the
/// signature line → join body onto the signature line (`= body`).
///
/// Conservative: only merges single-line signatures with a genuine `=`
/// assignment (tree-sitter can mis-parse `= a == b` into `=` tokens), and only
/// when the body expression starts on the next line.
fn fix_function_body_merge(source: &str, max_line_length: usize) -> String {
    let mut parser = KotlinParser::new();
    let tree = parser.parse(source);
    if tree.root_node().has_error() {
        return source.to_string();
    }
    // (start, end) byte ranges of whitespace runs to replace with a single space.
    let mut edits: Vec<(usize, usize)> = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.kind() == "function_declaration" {
            let body = {
                let mut w = node.walk();
                let x = node.children(&mut w).find(|c| c.kind() == "function_body");
                x
            };
            if let Some(body) = body {
                let mut bw = body.walk();
                let mut it = body.children(&mut bw);
                if let Some(eq) = it.next().filter(|n| n.kind() == "=") {
                    if let Some(expr) = it.next() {
                        if expr.start_position().row > eq.start_position().row
                            && node.start_position().row == eq.start_position().row
                            && &source[eq.start_byte()..eq.end_byte()] == "="
                            && source[..eq.start_byte()].chars().next_back() != Some('=')
                            && source[eq.end_byte()..].chars().next() != Some('=')
                            && expr.start_position().row == eq.start_position().row + 1
                        {
                            let func_start = node.start_byte();
                            let ls = source[..func_start].rfind('\n').map_or(0, |i| i + 1);
                            let indent_len = source[ls..func_start].chars().count();
                            let sig_len =
                                indent_len + source[func_start..eq.end_byte()].chars().count();
                            let params_node = {
                                let mut pw = node.walk();
                                let x = node
                                    .children(&mut pw)
                                    .find(|c| c.kind() == "function_value_parameters");
                                x
                            };
                            let has_params = params_node.is_some_and(|p| {
                                let mut pw = p.walk();
                                let any =
                                    p.children(&mut pw).any(|c| c.kind() == "value_parameter");
                                any
                            });
                            let remaining = if has_params && sig_len > max_line_length {
                                max_line_length.saturating_sub(eq.end_position().column)
                            } else {
                                max_line_length.saturating_sub(sig_len)
                            };
                            let body_start = expr.start_byte();
                            let body_line_end = source[body_start..]
                                .find('\n')
                                .map_or(source.len(), |i| body_start + i);
                            let first_line = &source[body_start..body_line_end];
                            let first_line_len = first_line.len();
                            if first_line_len < remaining
                                && !first_line.trim_start().starts_with('@')
                                && !source[body_start..expr.end_byte()].contains("\"\"\"")
                                && source[eq.end_byte()..body_start]
                                    .chars()
                                    .all(|c| c.is_whitespace())
                            {
                                edits.push((eq.end_byte(), body_start));
                            }
                        }
                    }
                }
            }
        }
        let mut w2 = node.walk();
        for c in node.children(&mut w2) {
            stack.push(c);
        }
    }
    if edits.is_empty() {
        return source.to_string();
    }
    // Apply from the end of the file backwards so earlier byte offsets stay
    // valid (edits are collected in DFS order, which is not file order).
    edits.sort_by_key(|(start, _)| std::cmp::Reverse(*start));
    let mut text = source.to_string();
    for (start, end) in edits {
        text.replace_range(start..end, " ");
    }
    text
}

/// `val x: Type = foo(...)` on a single overlong line → newline after `=`.
/// Mirrors PropertyWrappingRule: when the line up to the call expression
/// exceeds the limit, break after `=` (or before the call).
fn fix_property_wrapping(source: &str, indent_size: usize, max_line_length: usize) -> String {
    let mut parser = KotlinParser::new();
    let tree = parser.parse(source);
    if tree.root_node().has_error() {
        return source.to_string();
    }
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.kind() == "property_declaration" {
            if node.start_position().row != node.end_position().row {
                let mut w2 = node.walk();
                for c in node.children(&mut w2) {
                    stack.push(c);
                }
                continue;
            }
            let line_start = source[..node.start_byte()].rfind('\n').map_or(0, |i| i + 1);
            let line_end = source[node.end_byte()..]
                .find('\n')
                .map_or(source.len(), |i| node.end_byte() + i);
            if line_end - line_start <= max_line_length {
                let mut w2 = node.walk();
                for c in node.children(&mut w2) {
                    stack.push(c);
                }
                continue;
            }
            let mut nw = node.walk();
            let children: Vec<tree_sitter::Node> = node.children(&mut nw).collect();
            let eq_idx = children.iter().position(|c| c.kind() == "=");
            if let Some(eq_idx) = eq_idx {
                let eq = children[eq_idx];
                if let Some(rhs) = children[eq_idx + 1..].iter().find(|c| !c.kind().is_empty()) {
                    if source[eq.end_byte()..rhs.start_byte()]
                        .chars()
                        .all(|c| c.is_whitespace())
                    {
                        let line_text = &source[line_start..line_end];
                        let new_indent = line_text.len() - line_text.trim_start().len();
                        let repl = format!("\n{}", " ".repeat(new_indent + indent_size));
                        edits.push((eq.end_byte(), rhs.start_byte(), repl));
                    }
                }
            }
        }
        let mut w2 = node.walk();
        for c in node.children(&mut w2) {
            stack.push(c);
        }
    }
    if edits.is_empty() {
        return source.to_string();
    }
    let mut text = source.to_string();
    for (start, end, repl) in edits.into_iter().rev() {
        text.replace_range(start..end, &repl);
    }
    text
}

/// Single-line argument list exceeding the limit → each argument on its own
/// line; `)` aligned with the opening line's indent. Mirrors
/// ArgumentListWrappingRule autocorrect.
fn fix_argument_list_wrapping(source: &str, indent_size: usize, max_line_length: usize) -> String {
    let mut parser = KotlinParser::new();
    let tree = parser.parse(source);
    if tree.root_node().has_error() {
        return source.to_string();
    }
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "value_arguments" | "function_value_parameters") {
            if node.start_position().row == node.end_position().row {
                let line_start = source[..node.start_byte()].rfind('\n').map_or(0, |i| i + 1);
                let line_end = source[node.end_byte()..]
                    .find('\n')
                    .map_or(source.len(), |i| node.end_byte() + i);
                if line_end - line_start > max_line_length {
                    let line_text = &source[line_start..line_end];
                    let indent = line_text.len() - line_text.trim_start().len();
                    let arg_indent = indent + indent_size;
                    let mut cw = node.walk();
                    let children: Vec<tree_sitter::Node> = node.children(&mut cw).collect();
                    let has_lambda = children.iter().any(|c| {
                        let text = &source[c.start_byte()..c.end_byte()];
                        text.trim_start().starts_with('{') || text.trim_end().ends_with('}')
                    });
                    if has_lambda {
                        let mut w2 = node.walk();
                        for c in node.children(&mut w2) {
                            stack.push(c);
                        }
                        continue;
                    }
                    let args: Vec<tree_sitter::Node> = children
                        .iter()
                        .copied()
                        .filter(|c| !matches!(c.kind(), "(" | ")" | ","))
                        .collect();
                    if args.is_empty() {
                        let mut w2 = node.walk();
                        for c in node.children(&mut w2) {
                            stack.push(c);
                        }
                        continue;
                    }
                    for arg in &args {
                        if source[arg.start_byte()..arg.end_byte()]
                            .chars()
                            .all(|c| c.is_whitespace())
                        {
                            continue;
                        }
                        let prev_end = arg
                            .prev_sibling()
                            .map_or(node.start_byte() + 1, |p| p.end_byte());
                        if source[prev_end..arg.start_byte()]
                            .chars()
                            .all(|c| c.is_whitespace())
                        {
                            edits.push((
                                prev_end,
                                arg.start_byte(),
                                format!("\n{}", " ".repeat(arg_indent)),
                            ));
                        }
                    }
                    if let Some(rp) = children.iter().find(|c| c.kind() == ")") {
                        let prev = rp
                            .prev_sibling()
                            .map_or(node.start_byte() + 1, |p| p.end_byte());
                        if source[prev..rp.start_byte()]
                            .chars()
                            .all(|c| c.is_whitespace())
                        {
                            edits.push((
                                prev,
                                rp.start_byte(),
                                format!("\n{}", " ".repeat(indent)),
                            ));
                        }
                    }
                }
            }
        }
        let mut w2 = node.walk();
        for c in node.children(&mut w2) {
            stack.push(c);
        }
    }
    if edits.is_empty() {
        return source.to_string();
    }
    edits.sort_by_key(|(start, _, _)| *start);
    let mut text = source.to_string();
    for (start, end, repl) in edits.into_iter().rev() {
        text.replace_range(start..end, &repl);
    }
    text
}

/// Expand single-line brace blocks (`fun main() { println("hi") }` → the body
/// on its own line), mirroring ktlint's StatementWrappingRule autocorrect.
/// Same exclusions as the rule: empty/comment-only blocks, single-line
/// lambdas and enums, and tree-sitter's mis-parsed `by remember(...) { }`.
fn fix_statement_wrapping(source: &str, indent_size: usize) -> String {
    let mut parser = KotlinParser::new();
    let tree = parser.parse(source);
    if tree.root_node().has_error() {
        return source.to_string();
    }
    // Fence off string/char-literal and comment interiors so braces and
    // semicolons inside them are never touched. Re-parse the masked text so
    // node byte offsets match the text we edit.
    let Some((masked, store)) = mask_protected(source, &tree) else {
        return source.to_string();
    };
    let tree = parser.parse(&masked);
    let source = &masked;
    // (start, end, replacement) — replace the whitespace run after `{` or
    // before `}` with a newline + indent.
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
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
            collect_statement_wrapping_edits(&node, source, indent_size, &mut edits);
        }
        for i in (0..node.child_count()).rev() {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
    let mut text = source.to_string();
    if !edits.is_empty() {
        edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
        for (start, end, repl) in edits {
            text.replace_range(start..end, &repl);
        }
    }
    text = fix_semicolon_newlines(&text, indent_size);
    restore_protected(&text, &store)
}

fn collect_statement_wrapping_edits(
    node: &tree_sitter::Node,
    source: &str,
    indent_size: usize,
    edits: &mut Vec<(usize, usize, String)>,
) {
    let start = node.start_byte();
    let end = node.end_byte();
    let text = &source[start..end];

    // Exclusions mirroring the rule's check_block.
    if node.kind() == "enum_class_body" && node.start_position().row == node.end_position().row {
        return;
    }
    if node.kind() == "function_body" {
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
                            if source[prev.start_byte()..prev.end_byte()].contains('{') {
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
    // Empty / comment-only blocks are allowed.
    if next_code_token_sw(source, lbrace + 1, rbrace).is_none() {
        return;
    }
    // The block must be on a single line for this expansion.
    let block_line = source[..lbrace].matches('\n').count();
    if source[lbrace..rbrace].contains('\n') {
        return;
    }
    // Indent of the line containing the `{` (or `->`).
    let line_start = source[..lbrace].rfind('\n').map_or(0, |i| i + 1);
    let indent = source[line_start..lbrace]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .count();
    // Newline + indent after `{`.
    let first = next_code_token_sw(source, lbrace + 1, rbrace).unwrap();
    let after_lbrace = &source[lbrace + 1..first.0];
    if !after_lbrace.is_empty() {
        edits.push((
            lbrace + 1,
            first.0,
            format!("\n{}", " ".repeat(indent + indent_size)),
        ));
    }
    // Newline + indent before `}`.
    if let Some(prev) = prev_code_token_sw(source, lbrace + 1, rbrace) {
        let before_rbrace = &source[prev.1..rbrace];
        if !before_rbrace.is_empty() {
            edits.push((prev.1, rbrace, format!("\n{}", " ".repeat(indent))));
        }
    }
    let _ = block_line;
}

fn next_code_token_sw(source: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let mut i = start;
    while i < end {
        let c = source[i..].chars().next()?;
        if c.is_whitespace() {
            i += c.len_utf8();
            continue;
        }
        if source[i..].starts_with("//") || source[i..].starts_with("/*") {
            if source[i..].starts_with("/*") {
                let close = source[i + 2..].find("*/").map_or(end, |j| i + 2 + j + 2);
                i = close;
            } else {
                let nl = source[i..].find('\n').map_or(end, |j| i + j);
                i = nl;
            }
            continue;
        }
        return Some((i, i + c.len_utf8()));
    }
    None
}

fn prev_code_token_sw(source: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let mut chars = source[start..end].char_indices().rev();
    while let Some((rel, c)) = chars.next() {
        let i = start + rel;
        if c.is_whitespace() {
            continue;
        }
        if source[..i].ends_with("*/") {
            continue;
        }
        return Some((i, i + c.len_utf8()));
    }
    None
}

/// `;` separating statements on one line → newline after the `;`.
fn fix_semicolon_newlines(source: &str, indent_size: usize) -> String {
    let bytes = source.as_bytes();
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        // Advance over multi-byte characters (masked SENTINELs, non-ASCII) so
        // byte indexes stay on char boundaries.
        if bytes[i] >= 128 {
            let c = source[i..].chars().next().unwrap();
            i += c.len_utf8();
            continue;
        }
        if bytes[i] == b'"' {
            if source[i..].starts_with("\"\"\"") {
                let close = source[i + 3..]
                    .find("\"\"\"")
                    .map_or(bytes.len(), |j| i + 3 + j + 3);
                i = close;
                continue;
            }
            let close = source[i + 1..]
                .find('"')
                .map_or(bytes.len(), |j| i + 1 + j + 1);
            i = close;
            continue;
        }
        if bytes[i] == b'\'' {
            let close = source[i + 1..]
                .find('\'')
                .map_or(bytes.len(), |j| i + 1 + j + 1);
            i = close;
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
            // Already followed by a newline (possibly after spaces) — the
            // statement is separated; nothing to expand. This also covers the
            // trailing `;` of enum class bodies.
            let mut k = i + 1;
            while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b'\n' {
                i += 1;
                continue;
            }
            let after = source[i + 1..].trim_start();
            if !after.starts_with('}') && !after.is_empty() && !after.starts_with("//") {
                let line_start = source[..i].rfind('\n').map_or(0, |j| j + 1);
                let indent = source[line_start..i]
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .count();
                // Replace the `;` (and trailing space) with a newline —
                // ktlint removes the separator when expanding to two lines.
                edits.push((
                    i,
                    i + 1 + (source[i + 1..].len() - after.len()),
                    format!("\n{}", " ".repeat(indent)),
                ));
            }
        }
        i += 1;
    }
    if edits.is_empty() {
        return source.to_string();
    }
    edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    let mut text = source.to_string();
    for (start, end, repl) in edits {
        text.replace_range(start..end, &repl);
    }
    text
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

    // AST-driven expected indentation: for every code line, the containing
    // block's depth (class_body/function_body/control_structure_body/...)
    // determines the expected leading spaces. We only *raise* lines whose
    // current indent is clearly too shallow for their block — never lower
    // existing indentation, so continuation/wrapped lines are preserved.
    let mut expected: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    collect_expected_indents(&masked, &mut expected, indent_size);
    let lines: Vec<&str> = masked.split_inclusive('\n').collect();
    let mut output = String::with_capacity(masked.len());
    for (row, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with(SENTINEL) {
            output.push_str(line);
            continue;
        }
        let current = line.len() - trimmed.len();
        if let Some(&want) = expected.get(&row) {
            if current < want && current < want.saturating_sub(indent_size).max(1) {
                let has_nl = line.ends_with('\n');
                output.push_str(&" ".repeat(want));
                output.push_str(trimmed.trim_end_matches('\n'));
                if has_nl {
                    output.push('\n');
                }
                continue;
            }
        }
        output.push_str(line);
    }
    let mut result = output;
    if source.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    restore_protected(&result, &store)
}

/// Depth-first: every block-like node raises the expected indent of the lines
/// between its opening and closing brace by one level. Continuation lines
/// (those at depth>0 but not starting a new statement) are left alone because
/// we only ever raise clearly-too-shallow lines.
fn collect_expected_indents(
    masked: &str,
    expected: &mut std::collections::HashMap<usize, usize>,
    indent_size: usize,
) {
    // Brace-depth (+ paren-continuation) expected indent over the
    // protected-masked source (string interiors are SENTINEL). A line's
    // expected indent reflects braces closed at its start; its own braces then
    // update depth. Lines inside unclosed parens are continuations (no
    // expected — the fixer leaves them alone).
    let mut depth = 0usize;
    let mut parens = 0usize;
    for (row, line) in masked.split('\n').enumerate() {
        let trimmed = line.trim();
        let opens = trimmed.bytes().filter(|b| *b == b'{').count();
        // Only a line that *starts* with `}` (a closer / `} else` continuation)
        // sits one level shallower; interior braces belong to the same line.
        let leading_closes = trimmed.bytes().take_while(|b| *b == b'}').count();
        let po = trimmed.bytes().filter(|b| *b == b'(').count();
        let pc = trimmed.bytes().filter(|b| *b == b')').count();
        let line_depth = depth.saturating_sub(leading_closes);
        if parens == 0 {
            let entry = expected.entry(row).or_insert(0);
            let line_expected = line_depth * indent_size;
            if *entry < line_expected {
                *entry = line_expected;
            }
        }
        // Net depth: leading closers pop; this line's opens push; interior
        // braces (`x { ... }` on one line) pair and cancel.
        let total_closes = trimmed.bytes().filter(|b| *b == b'}').count();
        let interior_closes = total_closes - leading_closes;
        let net = opens.saturating_sub(interior_closes);
        depth = depth.saturating_sub(leading_closes).saturating_add(net);
        parens = parens.saturating_add(po).saturating_sub(pc);
    }
}

fn fix_trailing_ws(source: &str) -> String {
    // Trim trailing whitespace per line while preserving the newline structure.
    // split_inclusive keeps each line's terminator, so we must strip it first,
    // trim the content, then re-attach it — otherwise trim_end_matches stops at
    // the '\n' and leaves trailing spaces on `{ \n`-style lines untouched.
    source
        .split_inclusive('\n')
        .map(|line| {
            let (content, nl) = match line.strip_suffix('\n') {
                Some(c) => (c, "\n"),
                None => (line, ""),
            };
            let trimmed = content.trim_end_matches([' ', '\t']);
            format!("{trimmed}{nl}")
        })
        .collect::<String>()
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

fn fix_dot_spacing(source: &str) -> String {
    // Line-aware: strip whitespace before a dot only when the dot is NOT the
    // first token on its line (chained-call leading dots must keep their indent).
    // Also collapse multi-space after a dot.
    let mut out = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        let trimmed_start = line.trim_start();
        let leading_ws_len = line.len() - trimmed_start.len();
        let (head, rest) = line.split_at(leading_ws_len);
        out.push_str(head);
        let mut rest = rest.to_string();
        if !rest.starts_with('.') {
            rest = rest.replace(" .", ".");
        }
        rest = rest.replace(".  ", ". ");
        out.push_str(&rest);
    }
    out
}

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
    // Add a space after `{` only when followed by a non-whitespace character on
    // the same line (ktlint's curly-spacing). A `{` at end of line (`{\n`) or
    // followed by whitespace must stay untouched — inserting a space there creates
    // a trailing-space violation and breaks idempotence.
    let mut next = String::with_capacity(s.len());
    for (i, ch) in s.char_indices() {
        next.push(ch);
        if ch == '{' {
            let after = s[i + ch.len_utf8()..].chars().next();
            if matches!(after, Some(c) if !c.is_whitespace() && c != '\n') {
                next.push(' ');
            }
        }
    }
    s = next;
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
    // Import lines (`import foo.bar.*`) contain operator chars (`*`) that are
    // wildcards, not operators — leave them untouched.
    if source
        .lines()
        .any(|l| l.trim_start().starts_with("import "))
    {
        // Fast path: only reflow non-import lines; comment/KDoc lines and
        // string content are left untouched.
        let mut out = String::with_capacity(source.len());
        for line in source.split_inclusive('\n') {
            let t = line.trim_start();
            if t.starts_with("import ")
                || t.starts_with("//")
                || t.starts_with("/*")
                || t.starts_with("*")
                || t.starts_with('/')
            {
                out.push_str(line);
            } else {
                out.push_str(&fix_operators_inner(line));
            }
        }
        return out;
    }
    fix_operators_inner(source)
}

fn fix_operators_inner(source: &str) -> String {
    // Guard ranges (`0..<n`, `a..b`) from operator spacing by tokenizing them
    // away before the operator pass and restoring afterwards (inserts shift
    // byte offsets, so a precomputed range list is unsafe).
    let mut s = source.to_string();
    let mut range_tokens: Vec<(String, String)> = Vec::new();
    for token in ["..<", ".."] {
        while let Some(pos) = s.find(token) {
            let placeholder = format!("\u{E001}RG{}\u{E001}", range_tokens.len());
            s.replace_range(pos..pos + token.len(), &placeholder);
            range_tokens.push((placeholder.clone(), token.to_string()));
        }
    }
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
    for (placeholder, token) in range_tokens {
        s = s.replace(&placeholder, &token);
    }
    s
}

fn fix_commas(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut output = String::with_capacity(source.len());
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != ',' {
            output.push(chars[index]);
            index += 1;
            continue;
        }
        while output.ends_with(' ') || output.ends_with('\t') {
            output.pop();
        }
        output.push(',');
        index += 1;
        while matches!(chars.get(index), Some(' ' | '\t')) {
            index += 1;
        }
        if !matches!(chars.get(index), None | Some('\n' | '\r' | ')' | ']' | '>')) {
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

fn fix_double_colons(source: &str) -> String {
    source.replace(" ::", "::").replace(":: ", "::")
}

fn fix_colons(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        let chars: Vec<char> = line.chars().collect();
        let mut index = 0;
        while index < chars.len() {
            if chars[index] != ':'
                || chars.get(index + 1) == Some(&':')
                || (index > 0 && chars[index - 1] == ':')
            {
                output.push(chars[index]);
                index += 1;
                continue;
            }
            let prefix: String = chars[..index].iter().collect();
            let word_start = prefix
                .char_indices()
                .rev()
                .find(|(_, ch)| !ch.is_alphanumeric() && *ch != '_')
                .map_or(0, |(offset, ch)| offset + ch.len_utf8());
            if prefix[..word_start].ends_with('@') {
                output.push(':');
                index += 1;
                continue;
            }
            while output.ends_with(' ') || output.ends_with('\t') {
                output.pop();
            }
            let trimmed = line.trim_start();
            // A super-type colon (`class X(...) : Base`, `object : Y`) gets a
            // space before it; a constructor/property parameter colon
            // (`private val activity : X`) must not. So only add the space when
            // the colon follows a closing paren or generic bracket.
            // A super-type colon (`class Foo(...) : Base`, `object : Y`) gets a
            // space before it. Function return types (`fun foo() : Ret`) and
            // constructor/property parameter colons must NOT — verified against
            // real ktlint 1.8.0.
            let is_declaration = trimmed.starts_with("class ")
                || trimmed.starts_with("data class ")
                || trimmed.starts_with("enum class ")
                || trimmed.starts_with("sealed class ")
                || trimmed.starts_with("interface ")
                || trimmed.starts_with("object ");
            let prev_char = prefix.chars().rev().find(|c| !c.is_whitespace());
            // Super-type colon: after the primary constructor's `)` (class Foo
            // ...(..) : Base), after a generic `>` (class Foo<T> : Base), or an
            // object/interface without parens (object Bar : Base).
            let direct_object = (trimmed.starts_with("object ")
                || trimmed.starts_with("interface "))
                && prefix
                    .trim_end()
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_');
            let super_colon =
                (is_declaration && matches!(prev_char, Some(')' | '>'))) || direct_object;
            let type_constraint =
                prefix.rfind('<') > prefix.rfind('>') || prefix.contains(" where ");
            if super_colon || type_constraint {
                output.push(' ');
            }
            output.push(':');
            index += 1;
            while matches!(chars.get(index), Some(' ' | '\t')) {
                index += 1;
            }
            if !matches!(chars.get(index), None | Some('\n' | '\r')) {
                output.push(' ');
            }
        }
    }
    output
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
        let trimmed = line.trim_start();
        // A getter/setter annotation (`@Composable get()`) belongs to the
        // property above it — never separate it with a blank line.
        let is_accessor_annotation =
            trimmed.starts_with('@') && (trimmed.contains("get(") || trimmed.contains("set("));
        if is_annotation_only_line(trimmed) && index > 0 && !is_accessor_annotation {
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

fn fix_expression_operand_wrapping(source: &str) -> String {
    let mut output = String::with_capacity(source.len() + 8);
    for line in source.split_inclusive('\n') {
        if let Some(operator_end) =
            crate::rules::wrapping::compatibility::unwrapped_operand_after_operator(line)
        {
            // Only wrap when a real operand follows the operator on the same
            // line (`a * b` -> `a *\n    b`). A line that already ends with an
            // operator has no operand to move — leave it untouched.
            let operand_start = line[operator_end..]
                .find(|character: char| !character.is_whitespace())
                .map_or(operator_end, |offset| operator_end + offset);
            if operand_start >= line.len().saturating_sub(1) {
                output.push_str(line);
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            output.push_str(&line[..operator_end]);
            output.push('\n');
            output.push_str(&" ".repeat(indent + 4));
            output.push_str(&line[operand_start..]);
        } else {
            output.push_str(line);
        }
    }
    output
}

fn fix_empty_class_body(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        if let Some(opening) =
            crate::rules::structure::no_empty_class_body::empty_declaration_body(line)
        {
            let closing = line.rfind('}').unwrap_or(opening);
            output.push_str(line[..opening].trim_end());
            output.push_str(&line[closing + 1..]);
        } else {
            output.push_str(line);
        }
    }
    output
}

fn fix_parameter_list_wrapping(source: &str) -> String {
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut output = String::with_capacity(source.len() + 16);
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();
        let declaration = trimmed.starts_with("fun ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("data class ")
            || trimmed.starts_with("constructor(");
        let opening = declaration.then(|| line.find('(')).flatten();
        if let Some(opening) = opening {
            if !line[opening + 1..].contains(')')
                && !line[opening + 1..].trim().is_empty()
                && index + 1 < lines.len()
            {
                if let Some(closing) = lines[index + 1].find(')') {
                    let first = line[opening + 1..].trim();
                    let second = lines[index + 1][..closing].trim().trim_end_matches(',');
                    output.push_str(&line[..=opening]);
                    output.push_str(first);
                    output.push(' ');
                    output.push_str(second);
                    output.push_str(&lines[index + 1][closing..]);
                    index += 2;
                    continue;
                }
            }
        }
        output.push_str(line);
        index += 1;
    }
    output
}

fn fix_context_receiver_list_wrapping(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        if let Some(declaration_start) =
            crate::rules::wrapping::compatibility::context_declaration_on_same_line(line)
        {
            let indent = line.len() - line.trim_start().len();
            let closing = line[..declaration_start].trim_end().len();
            output.push_str(&line[..closing]);
            output.push('\n');
            output.push_str(&" ".repeat(indent));
            output.push_str(&line[declaration_start..]);
        } else {
            output.push_str(line);
        }
    }
    output
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
            let comment_text = &source[node.byte_range()];
            // License headers (`/*\n* Copyright ... */`) keep their own style —
            // the `*` column is managed by the license tooling, not ktlint.
            let is_license = comment_text.to_ascii_lowercase().contains("copyright");
            let expected = " ".repeat(node.start_position().column + 1);
            let text = &source[node.byte_range()];
            let mut absolute = node.start_byte();
            for (line_index, line) in text.split_inclusive('\n').enumerate() {
                if line_index > 0 && !is_license {
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
    ["if", "for", "while", "when", "catch", "fun"]
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
            // Only strip `()` before a trailing lambda when the callee is a
            // function-style lowercase identifier (`listOf(1).forEach() { }`).
            // A capitalized name is a constructor call (`ViewModel() { }`),
            // where the parens are required — stripping them corrupts syntax.
            let callee = line
                .split('(')
                .next()
                .and_then(|head| {
                    head.rsplit(|c: char| !c.is_alphanumeric() && c != '_')
                        .next()
                })
                .unwrap_or("");
            let lowercase_callee = callee
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_lowercase());
            let trimmed = line.trim_start();
            let is_decl_or_accessor = trimmed.starts_with("fun ")
                || trimmed.contains(" fun ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("interface ")
                || trimmed.starts_with("object ")
                || trimmed.contains("get() {")
                || trimmed.contains("set(");
            let is_delegation =
                line.contains("this(") || line.contains("super(") || line.contains("constructor(");
            if lowercase_callee && !is_decl_or_accessor && !is_delegation {
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
            // Only strip a trailing semicolon when it is real code, not inside
            // a string/char literal or a comment (e.g. `println("a;b")` or
            // `// foo;`).
            if trimmed != ";" && line.trim_end().ends_with(';') {
                let code = line.trim_end();
                let semi = code.rfind(';').unwrap_or(0);
                let prefix = &code[..semi];
                let in_string = prefix.matches('"').count() % 2 == 1
                    || prefix.matches("\"\"\"").count() % 2 == 1;
                let in_comment = prefix.contains("//") || prefix.contains("/*");
                // An enum entry separator (`CLOSE;` before member functions)
                // is required syntax — never strip it.
                let enum_name = prefix.trim();
                // Enum entries: bare identifier (`CLOSE;`), uppercase constant
                // (`CONNECT;`), or with args (`ApplicationData(0x17);`).
                // Anything with statement keywords is real code.
                let is_enum_entry = {
                    let first = enum_name.chars().next().unwrap_or(' ');
                    let has_statement_kw =
                        ["return", "val ", "var ", "fun ", "if ", "for ", "while "]
                            .iter()
                            .any(|kw| enum_name.starts_with(kw));
                    !has_statement_kw
                        && (first.is_ascii_uppercase()
                            || first.is_ascii_lowercase()
                            || first == '_')
                        && enum_name.ends_with(')')
                        || (first.is_ascii_uppercase()
                            && enum_name
                                .chars()
                                .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
                            && !enum_name.is_empty())
                };
                if !in_string && !in_comment && !is_enum_entry {
                    code.trim_end_matches(';').to_string()
                } else {
                    line.to_string()
                }
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
            // `class ` also matches `::class` (callable reference) inside
            // expressions (`this::class == ...`) — those lines must not have
            // their `):` (e.g. a function return type) rewritten.
            if !line.contains("class ") || line.contains("::class") {
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

/// `standard:blank-line-between-when-conditions` — insert a blank line before
/// a when-branch whose body is a block (`else -> {`), separating it from a
/// preceding simple-expression branch, mirroring ktlint 1.8.
fn fix_when_conditions_blank_lines(source: &str) -> String {
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut out: Vec<&str> = Vec::with_capacity(lines.len());
    let mut in_when = false;
    let mut prev_was_branch = false;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !in_when {
            let has_when = trimmed
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|w| w == "when");
            if has_when && trimmed.contains('{') && !trimmed.contains('}') {
                in_when = true;
                prev_was_branch = false;
            }
            out.push(line);
            continue;
        }
        if trimmed == "}" {
            in_when = false;
            out.push(line);
            continue;
        }
        if trimmed.contains("->") {
            let this_is_block = trimmed.ends_with('{');
            if (prev_was_branch || this_is_block)
                && i > 0
                && !out.last().is_some_and(|l| l.trim().is_empty())
            {
                // Insert a blank line before this branch (unless one exists).
                out.push("\n");
            }
            prev_was_branch = this_is_block;
        } else if !trimmed.is_empty() && !trimmed.starts_with('}') {
            // Branch block body content; the block branch itself was already
            // separated.
        }
        out.push(line);
    }
    out.concat()
}

fn fix_blank_lines(source: &str) -> String {
    let mut s = source.to_string();
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }
    // Collapse blank lines immediately before a closing brace on its own line
    // (standard:no-blank-line-before-rbrace): `\n\n}` → `\n}`.
    let mut next = String::with_capacity(s.len());
    let lines: Vec<&str> = s.split_inclusive('\n').collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let next_line = lines.get(i + 1).copied().unwrap_or("");
        if line.trim().is_empty() && next_line.trim_start().starts_with('}') {
            i += 1; // drop the blank line before the closing brace
            continue;
        }
        next.push_str(line);
        i += 1;
    }
    s = next;
    // Collapse trailing blank lines: keep at most one final newline.
    let trimmed_end = s.trim_end_matches('\n');
    if trimmed_end.len() != s.len() {
        s = format!("{trimmed_end}\n");
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
        let once = format_once(
            source,
            4,
            true,
            &HashMap::new(),
            CodeStyle::KtlintOfficial,
            120,
        )
        .unwrap();
        assert_eq!(
            format_once(
                &once,
                4,
                true,
                &HashMap::new(),
                CodeStyle::KtlintOfficial,
                120
            )
            .unwrap(),
            once
        );
    }

    #[test]
    fn trailing_blank_lines_are_idempotent() {
        // Trailing whitespace + trailing blank lines must not make the
        // no-trailing-spaces pass oscillate (regression: fix_trailing_ws used
        // lines()/join which dropped the final newline on round two).
        let source = "val x = 1   \n\n";
        let once = format_once(
            source,
            4,
            true,
            &HashMap::new(),
            CodeStyle::KtlintOfficial,
            120,
        )
        .unwrap();
        let twice = format_once(
            &once,
            4,
            true,
            &HashMap::new(),
            CodeStyle::KtlintOfficial,
            120,
        )
        .unwrap();
        assert_eq!(
            once, twice,
            "format_once must be idempotent on trailing blank lines"
        );
        assert!(!once.contains("   \n"), "trailing spaces must be trimmed");

        // CLI parity: Android Studio style, final newline on/off.
        for code_style in [CodeStyle::KtlintOfficial, CodeStyle::AndroidStudio] {
            for newline in [true, false] {
                let first =
                    format_once(source, 4, newline, &HashMap::new(), code_style, 120).unwrap();
                let second =
                    format_once(&first, 4, newline, &HashMap::new(), code_style, 120).unwrap();
                assert_eq!(
                    first, second,
                    "style={code_style:?} newline={newline}: must be idempotent"
                );
            }
        }
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
        let output = format_once(
            source,
            4,
            true,
            &HashMap::new(),
            CodeStyle::KtlintOfficial,
            120,
        )
        .unwrap();
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
            format_once(
                &output,
                4,
                true,
                &HashMap::new(),
                CodeStyle::KtlintOfficial,
                120
            )
            .unwrap(),
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
    #[test]
    fn fix_curly_brace() {
        assert_eq!(fix_all_spacing("fun foo(){x}"), "fun foo() { x }");
    }
    #[test]
    fn fix_colon_spacing() {
        assert!(fix_all_spacing("val x:String").contains("x: String"));
    }
    #[test]
    #[test]
    fn lambda_parens_stripped_for_calls_not_constructors() {
        let call = "fun lambda() {\n    listOf(1).forEach() { value -> println(value) }\n}\n";
        assert!(fix_trailing_lambda_parentheses(call).contains("forEach {"));
        // Declarations and constructor calls keep their parens.
        let decl = "fun foo() {\n}\n";
        assert!(fix_trailing_lambda_parentheses(decl).contains("fun foo() {"));
        let ctor = "class Foo : ViewModel() {\n}\n";
        assert!(fix_trailing_lambda_parentheses(ctor).contains("ViewModel() {"));
    }

    fn fix_trailing_ws_test() {
        assert_eq!(fix_trailing_ws("val x = 1   \n   "), "val x = 1\n");
    }
    #[test]
    #[test]
    fn expression_operand_wraps_line_end_operator() {
        // A line ending in an operator with an operand before it splits the
        // operand onto a continuation line; a line that already ends with the
        // operator (no operand after) stays put.
        let src = "val interaction =\n    (first + second) * third *\n        fourth\n";
        let out = fix_expression_operand_wrapping(src);
        assert!(
            out.contains("(first + second) *\n        third *\n"),
            "should split the trailing operand: {out:?}"
        );
        let end_op = "val x = a +\n    b\n";
        assert_eq!(fix_expression_operand_wrapping(end_op), end_op);
    }

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
        let source = "val x = foo.bar() { }\n";
        assert_eq!(
            fix_trailing_lambda_parentheses(source),
            "val x = foo.bar { }\n"
        );
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

    #[test]
    fn wrapping_fix_argument_list_expands_single_line_overlong() {
        let src = "package com.example\n\nval result = combineValues(firstValueName, secondValueName, thirdValueName, fourthValueName, fifthValueName, sixthValueName, seventhValueName)\n";
        let out = fix_wrapping(src, 4, 120);
        assert!(
            out.contains("combineValues(\n"),
            "should open after callee: {out}"
        );
        assert!(
            out.contains("\n        firstValueName,"),
            "first arg on own line: {out}"
        );
        assert!(
            out.contains("\n        secondValueName,"),
            "second arg on own line: {out}"
        );
        assert!(
            out.contains("\n    )"),
            "closing paren at opening indent: {out}"
        );
    }

    #[test]
    fn wrapping_fix_body_merge_joins_fitting_body() {
        let src = "package com.example\n\nfun build(extra: Array<Pair<String, String>>): Map<String, String> =\n    mapOf(\n        *extra,\n    )\n";
        let out = fix_wrapping(src, 4, 120);
        assert!(
            out.contains("Map<String, String> = mapOf("),
            "body should join signature line: {out}"
        );
    }

    #[test]
    fn wrapping_fix_property_breaks_overlong_line_after_equal() {
        let src =
            "package com.example\n\nval result = combineValues(firstValueName, secondValueName, thirdValueName, fourthValueName, fifthValueName, sixthValueName)\n";
        let out = fix_wrapping(src, 4, 120);
        assert!(
            out.contains("val result =\n    combineValues("),
            "property should break after =: {out}"
        );
    }

    #[test]
    fn wrapping_fix_leaves_short_code_untouched() {
        let src = "package com.example\n\nval ok = shortCall(a, b)\n";
        assert_eq!(fix_wrapping(src, 4, 120), src);
    }
}
