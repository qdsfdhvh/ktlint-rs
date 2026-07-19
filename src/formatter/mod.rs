//! Auto-fix formatter — applies text-level corrections for fixable violations.
use crate::rules::Violation;
use std::collections::HashSet;
use std::path::PathBuf;

pub fn auto_fix(files: &[PathBuf], violations: &[Violation]) -> anyhow::Result<()> {
    let fixable: Vec<&Violation> = violations.iter().filter(|v| v.auto_fixable).collect();
    if fixable.is_empty() { return Ok(()); }

    let mut file_set: HashSet<&str> = HashSet::new();
    for v in &fixable { if !v.file.is_empty() { file_set.insert(&v.file); } }

    for file_path in &file_set {
        let rules: Vec<&str> = fixable.iter().filter(|v| v.file == *file_path).map(|v| v.rule_id.as_str()).collect();
        let any_spacing = rules.iter().any(|r| r.contains("spacing") || r.contains("curly") || r.contains("op-") || r.contains("paren") || r.contains("colon") || r.contains("comma") || r.contains("comment"));
        let any_wrapping = rules.iter().any(|r| r.contains("wrapping"));
        let any_indent = rules.iter().any(|r| r.contains("indent"));

        let original = std::fs::read_to_string(file_path)?;
        let mut text = original.clone();
        if any_spacing { text = fix_all_spacing(&text); }
        if any_wrapping { text = fix_all_wrapping(&text); }
        if any_indent { text = fix_indentation(&text); }
        text = fix_trailing_ws(&text);
        if text != original { std::fs::write(file_path, text)?; }
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
        text = fix_colons(&text);
        text = fix_comment_spacing(&text);
        text = fix_blank_lines(&text);
        text = fix_brace_between(&text);
        text = fix_double_spaces(&text);
        if text == before { break; }
    }
    text
}

fn fix_all_wrapping(source: &str) -> String {
    let mut text = source.to_string();
    for _ in 0..3 {
        let before = text.clone();
        text = fix_multiline_if_else(&text);
        if text == before { break; }
    }
    text
}

fn fix_indentation(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    let mut indent = 0usize;
    let sz = 4;
    for line in &lines {
        let t = line.trim();
        if t.is_empty() { out.push(String::new()); continue; }
        if t.starts_with('}') || t == ")" { indent = indent.saturating_sub(sz); }
        out.push(format!("{}{}", " ".repeat(indent), t));
        if t.ends_with('{') && !t.contains("//") && !t.contains("/*") { indent += sz; }
    }
    out.join("\n")
}

fn fix_trailing_ws(source: &str) -> String {
    source.lines().map(|l| l.trim_end()).collect::<Vec<_>>().join("\n")
}

// ── Spacing helpers ──

fn fix_curly_braces(source: &str) -> String {
    let mut s = source.to_string();
    let indices: Vec<usize> = s.match_indices('{').map(|(i, _)| i).collect();
    for &pos in indices.iter().rev() {
        if pos > 0 {
            let prev = s[..pos].chars().last().unwrap_or(' ');
            if prev != ' ' && prev != '\n' && prev != '(' {
                s.insert(pos, ' ');
            }
        }
    }
    for kw in &["else", "catch", "finally"] {
        s = s.replace(&format!("}}{}", kw), &format!("}} {}", kw));
    }
    s
}

fn fix_operators(source: &str) -> String {
    let mut s = source.to_string();
    let ops = ["==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "=", "+", "-", "*", "/", "%"];
    for op in &ops {
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if i > 0 && i + op.len() <= chars.len() {
                let rest: String = chars[i..i+op.len()].iter().collect();
                if rest == *op {
                    let prev = chars[i-1];
                    let next = chars.get(i + op.len()).copied().unwrap_or(' ');
                    if prev.is_alphanumeric() && prev != ' ' {
                        s.insert(i, ' ');
                        i += 1;
                    }
                    i += op.len();
                    if next.is_alphanumeric() && next != ' ' {
                        s.insert(i, ' ');
                    }
                }
            }
            i += 1;
        }
    }
    s
}

fn fix_commas(source: &str) -> String {
    let mut s = source.to_string();
    if !s.contains(", ") { s = s.replace(",", ", "); }
    s
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
        if chars[i] == ':' && chars[i-1].is_alphanumeric() && chars[i+1].is_alphanumeric() && chars[i+1] != ' ' {
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
    while s.contains("\n\n\n") { s = s.replace("\n\n\n", "\n\n"); }
    s
}

fn fix_brace_between(source: &str) -> String {
    source.replace("\n} else {", "} else {")
        .replace("\n} catch", "} catch")
        .replace("\n} finally", "} finally")
}

fn fix_double_spaces(source: &str) -> String {
    let mut s = source.to_string();
    while s.contains("  ") { s = s.replace("  ", " "); }
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
            let body = lines[i+1].trim();
            if !body.contains('{') && !body.contains("if") && body.len() < 60 {
                let replacement = format!("{} {}", t, body);
                result = result.replace(&format!("{}\n    {}", t, body), &replacement);
                result = result.replace(&format!("{}\n{}", lines[i], lines[i+1]), &replacement);
                break;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fix_operator_equals() { assert_eq!(fix_all_spacing("val x=1"), "val x = 1"); }
    #[test]
    fn fix_curly_brace() { assert_eq!(fix_all_spacing("fun foo(){x}"), "fun foo() {x}"); }
    #[test]
    fn fix_colon_spacing() { assert!(fix_all_spacing("val x:String").contains("x: String")); }
    #[test]
    fn fix_trailing_ws_test() { assert_eq!(fix_trailing_ws("val x = 1   \n   "), "val x = 1\n"); }
    #[test]
    fn fix_indent() {
        let r = fix_indentation("class Foo {\nval x = 1\n}");
        assert!(r.contains("    val x"), "got: {}", r);
    }
    #[test]
    fn fix_wrapping_preserves() {
        let src = "val x = foo\n    .bar()\n    .baz()";
        let r = fix_all_wrapping(src);
        assert_eq!(r, src);
    }
}
