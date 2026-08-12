//! @Suppress / @SuppressWarnings annotation support.
//!
//! ktlint 1.8 parity: a `@Suppress(...)` / `@SuppressWarnings(...)`
//! annotation on an element suppresses matching rules for that element
//! (its whole line span). Arguments match (JVM oracle, probed):
//! - `"ktlint"` — all rules
//! - the full id: `@Suppress("ktlint:standard:function-naming")`
//! - the IntelliJ inspection name alias (currently `"FunctionName"` for
//!   function-naming; extend as more appear)
//! - `@file:Suppress(...)` applies to the whole file.

use std::collections::HashSet;

/// One suppression range: rows [start, end] inclusive, and the set of
/// suppressed rule ids (empty set = all rules).
pub struct SuppressRange {
    pub start_row: usize,
    pub end_row: usize,
    pub rules: HashSet<String>,
}

/// IntelliJ inspection names that map to ktlint rule ids.
fn intellij_alias(arg: &str) -> Option<String> {
    match arg {
        "FunctionName" => Some("standard:function-naming".to_string()),
        _ => None,
    }
}

/// Collect suppression ranges from the CST: find `@Suppress`/`@SuppressWarnings`
/// annotations (including `@file:`) and resolve the annotated element's row
/// span (the declaration that carries the annotation).
pub fn collect_suppressions(
    tree: &tree_sitter::Tree,
    source: &str,
) -> Vec<SuppressRange> {
    let mut ranges: Vec<SuppressRange> = Vec::new();
    let mut stack: Vec<tree_sitter::Node> = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.kind() == "annotation" {
            let text = &source[node.start_byte()..node.end_byte()];
            let trimmed = text.trim();
            if trimmed.starts_with("@Suppress")
                || trimmed.starts_with("@SuppressWarnings")
                || trimmed.starts_with("@file:Suppress")
            {
                if let Some(args) = extract_args(trimmed) {
                    let rules = parse_args(&args);
                    let (start, end) = element_span(&node, source);
                    if !rules.is_empty() {
                        ranges.push(SuppressRange {
                            start_row: start,
                            end_row: end,
                            rules,
                        });
                    }
                }
            }
        }
        for i in (0..node.child_count()).rev() {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
    ranges
}

fn extract_args(text: &str) -> Option<String> {
    let open = text.find('(')?;
    let rest = &text[open + 1..];
    let close = rest.find(')')?;
    Some(rest[..close].to_string())
}

fn parse_args(args: &str) -> HashSet<String> {
    let mut rules = HashSet::new();
    for arg in args.split(',') {
        let arg = arg.trim().trim_matches('"').trim();
        if arg.is_empty() {
            continue;
        }
        if arg == "ktlint" {
            rules.insert(String::new()); // empty = all rules
        } else if let Some(rule) = arg.strip_prefix("ktlint:") {
            // `ktlint:standard:function-naming` — exact rule id.
            rules.insert(rule.to_string());
        } else if let Some(alias) = intellij_alias(arg) {
            rules.insert(alias);
        }
        // Anything else (bare `function-naming`, `FunctionNaming`, …) does
        // NOT match (JVM oracle: probed — only the IntelliJ inspection name
        // and the full `ktlint:…` id suppress).
    }
    rules
}

/// Row span of the element the annotation applies to: the enclosing
/// declaration's start..end rows, or just the annotation row when no
/// declaration encloses it (e.g. a lone `@file:` annotation).
fn element_span(node: &tree_sitter::Node, _source: &str) -> (usize, usize) {
    let mut n = *node;
    let mut best: Option<tree_sitter::Node> = None;
    while let Some(p) = n.parent() {
        match p.kind() {
            "function_declaration"
            | "property_declaration"
            | "class_declaration"
            | "object_declaration"
            | "enum_entry"
            | "type_alias_declaration"
            | "function_value_parameter"
            | "class_parameter" => {
                best = Some(p);
                break;
            }
            _ => {}
        }
        n = p;
    }
    if let Some(b) = best {
        (b.start_position().row, b.end_position().row)
    } else {
        (node.start_position().row, node.start_position().row)
    }
}
