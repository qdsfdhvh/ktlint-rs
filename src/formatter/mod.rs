//! Auto-fix formatter — applies text-level corrections for fixable violations.
use crate::rules::Violation;
use std::collections::HashSet;
use std::path::PathBuf;

pub fn auto_fix(_files: &[PathBuf], violations: &[Violation], indent_size: usize, insert_final_newline: bool) -> anyhow::Result<()> {
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
        let rules: Vec<&str> = fixable
            .iter()
            .filter(|v| v.file == *file_path)
            .map(|v| v.rule_id.as_str())
            .collect();
        let any_spacing = rules.iter().any(|r| {
            r.contains("spacing")
                || r.contains("curly")
                || r.contains("op-")
                || r.contains("paren")
                || r.contains("colon")
                || r.contains("comma")
                || r.contains("comment")
        });
        let any_wrapping = rules.iter().any(|r| {
            r.contains("wrapping") || r.contains("when-entry-bracing") || r.contains("try-catch")
        });
        let any_indent = rules.iter().any(|r| r.contains("indent"));

        let original = std::fs::read_to_string(file_path)?;
        let mut text = original.clone();
        if any_spacing {
            text = fix_all_spacing(&text);
        }
        if any_wrapping {
            text = fix_all_wrapping(&text);
        }
        if any_indent {
            text = fix_indentation(&text, indent_size);
        }
        text = fix_trailing_ws(&text);
        // Issue #45: restore final newline if config requires it
        if insert_final_newline && !text.ends_with('\n') {
            text.push('\n');
        }
        if text != original {
            std::fs::write(file_path, text)?;
        }
    }
    Ok(())
}

fn fix_all_spacing(source: &str) -> String {
    let mut text = source.to_string();
    for _ in 0..5 {
        let before = text.clone();
        text = fix_curly_braces(&text);
        text = fix_operators(&text);
        text = fix_commas(&text);
        text = fix_parens(&text);
        text = fix_angle_brackets(&text);
        text = fix_colons(&text);
        text = fix_comment_spacing(&text);
        text = fix_blank_lines(&text);
        text = fix_blank_line_in_list(&text);
        text = fix_brace_between(&text);
        text = fix_double_spaces(&text);
        if text == before {
            break;
        }
    }
    text
}

fn fix_all_wrapping(source: &str) -> String {
    let mut text = source.to_string();
    for _ in 0..3 {
        let before = text.clone();
        text = fix_multiline_if_else(&text);
        text = fix_chain_wrapping(&text);
        text = fix_when_expression_break(&text);
        text = fix_try_catch(&text);
        text = fix_when_entry_bracing(&text);
        text = fix_string_template(&text);
        if text == before {
            break;
        }
    }
    text
}

fn fix_indentation(source: &str, indent_size: usize) -> String {
    let lines: Vec<&str> = source.lines().collect();
    // Detect base indent from first non-empty line
    let base = lines
        .iter()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);
    let mut out = Vec::new();
    let mut level = 0usize; // indent level relative to base
    let sz = indent_size;
    for line in &lines {
        let t = line.trim();
        if t.is_empty() {
            out.push(String::new());
            continue;
        }
        // Decrease level BEFORE outputting closing brace
        if t.starts_with('}') {
            level = level.saturating_sub(1);
        }
        let actual = base + level * sz;
        out.push(format!("{}{}", " ".repeat(actual), t));
        // Increase level AFTER outputting opening brace
        if t.ends_with('{') && !t.contains("//") && !t.contains("/*") {
            level += 1;
        }
    }
    out.join("\n")
}

fn fix_trailing_ws(source: &str) -> String {
    source
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Spacing helpers ──

fn fix_curly_braces(source: &str) -> String {
    let mut s = source.to_string();
    let indices: Vec<usize> = s.match_indices('{').map(|(i, _)| i).collect();
    for &pos in indices.iter().rev() {
        if pos > 0 {
            let prev = s[..pos].chars().last().unwrap_or(' ');
            // Skip string interpolation ${}, annotations @X({}), and ({}
            if prev != ' ' && prev != '\n' && prev != '$' && prev != '(' && prev != '[' {
                s.insert(pos, ' ');
            }
        }
    }
    for kw in &["else", "catch", "finally"] {
        s = s.replace(&format!("}}{}", kw), &format!("}} {}", kw));
    }
    // Merge }\nelse if onto one line
    s = s.replace("}\nelse if", "} else if");
    s
}

fn fix_operators(source: &str) -> String {
    let mut s = source.to_string();
    let ops = [
        "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "=", "+", "-", "*", "/", "%",
    ];
    for op in &ops {
        // Phase 1: collect all positions
        let chars: Vec<char> = s.chars().collect();
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
                    if !is_unary_minus {
                        positions.push(i);
                    }
                }
            }
            i += 1;
        }
        // Phase 2: apply fixes right-to-left (indices stay stable)
        for &pos in positions.iter().rev() {
            let cur: Vec<char> = s.chars().collect();
            if pos >= cur.len() || pos + op.len() > cur.len() {
                continue;
            }
            let cur_rest: String = cur[pos..pos + op.len()].iter().collect();
            if cur_rest != *op {
                continue;
            }
            let prev = cur[pos - 1];
            let next = cur.get(pos + op.len()).copied().unwrap_or(' ');
            if prev.is_alphanumeric() && prev != ' ' {
                s.insert(pos, ' ');
            }
            // After insert, re-read positions
            let cur2: Vec<char> = s.chars().collect();
            let after = pos + op.len() + if prev.is_alphanumeric() && prev != ' ' { 1 } else { 0 };
            if after < cur2.len() {
                let actual_next = cur2[after];
                // Align with rule: insert unless next is space ) \n ,
                if actual_next != ' ' && actual_next != ')' && actual_next != '\n' && actual_next != ',' {
                    s.insert(after, ' ');
                }
            }
        }
    }
    s
}

fn fix_commas(source: &str) -> String {
    let mut s = source.to_string();
    if !s.contains(", ") {
        s = s.replace(",", ", ");
    }
    s
}

fn fix_angle_brackets(source: &str) -> String {
    source.replace("< ", "<").replace(" >", ">")
}

fn fix_parens(source: &str) -> String {
    source.replace("( ", "(").replace(" )", ")")
}

fn fix_colons(source: &str) -> String {
    let mut s = source.replace(" : ", ": ");
    // Fix word:word → word: word
    let chars: Vec<char> = s.chars().collect();
    let mut i = 1;
    while i < chars.len().saturating_sub(1) {
        if chars[i] == ':'
            && chars[i - 1].is_alphanumeric()
            && chars[i + 1].is_alphanumeric()
            && chars[i + 1] != ' '
        {
            s.insert(i + 1, ' ');
        }
        i += 1;
    }
    s
}

fn fix_comment_spacing(source: &str) -> String {
    source.replace("//", "// ")
}

fn fix_blank_lines(source: &str) -> String {
    let mut s = source.to_string();
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }
    s
}

fn fix_blank_line_in_list(source: &str) -> String {
    // Remove blank lines between list items (inside brackets)
    let lines: Vec<&str> = source.lines().collect();
    let mut result = Vec::new();
    let mut bracket_depth = 0i32;
    for (_i, line) in lines.iter().enumerate() {
        let t = line.trim();
        // Track bracket depth
        bracket_depth += t.chars().filter(|&c| c == '(' || c == '[').count() as i32;
        bracket_depth -= t.chars().filter(|&c| c == ')' || c == ']').count() as i32;

        // Remove empty lines inside brackets
        if t.is_empty() && bracket_depth > 0 {
            continue;
        }
        result.push(line.to_string());
    }
    result.join("\n")
}

fn fix_brace_between(source: &str) -> String {
    source
        .replace("\n} else {", "} else {")
        .replace("\n} else if", "} else if")
        .replace("\n} catch", "} catch")
        .replace("\n} finally", "} finally")
}

fn fix_double_spaces(source: &str) -> String {
    let mut s = source.to_string();
    while s.contains("  ") {
        s = s.replace("  ", " ");
    }
    s
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
    // Normalize dot-call chains: if any call is on its own line
    // (indented), ensure all are. If all on one line, keep.
    let lines: Vec<&str> = source.lines().collect();
    let mut has_multiline = false;
    for line in &lines {
        let t = line.trim();
        if t.starts_with('.') && !t.starts_with("..") {
            has_multiline = true;
            break;
        }
    }
    if !has_multiline {
        return source.to_string();
    }

    // Rebuild: put each .call() on its own indented line
    let mut result = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with('.') {
            result.push(format!("    {}", t));
        } else if t.ends_with(')') && result.last().map_or(false, |l| l.trim().starts_with('.')) {
            // Previous line was a dot call — continue
            let prev = result.pop().unwrap();
            result.push(format!("    {}.{}", prev.trim(), t));
        } else {
            result.push(line.to_string());
        }
    }
    result.join("\n")
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
    fn fix_operator_equals() {
        assert_eq!(fix_all_spacing("val x=1"), "val x = 1");
    }
    #[test]
    fn fix_curly_brace() {
        assert_eq!(fix_all_spacing("fun foo(){x}"), "fun foo() {x}");
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
        let r = fix_indentation("class Foo {\nval x = 1\n}", 4);
        assert!(r.contains("    val x"), "got: {}", r);
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
}
