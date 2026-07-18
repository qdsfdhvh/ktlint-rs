//! Auto-fix engine — applies corrections for spacing violations.
//! Uses text-level pattern replacement to fix common violations.

use crate::rules::Violation;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn auto_fix(_files: &[PathBuf], violations: &[Violation]) -> Result<()> {
    let mut by_file: HashMap<&str, Vec<&Violation>> = HashMap::new();
    for v in violations {
        if v.auto_fixable {
            by_file.entry(&v.file).or_default().push(v);
        }
    }

    for (file_path, _file_violations) in &by_file {
        let path = std::path::Path::new(file_path);
        let mut text = std::fs::read_to_string(path)?;
        let original = text.clone();

        let rules: std::collections::HashSet<&str> = _file_violations
            .iter()
            .map(|v| v.rule_id.as_str())
            .collect();
        let any_spacing = rules.iter().any(|r| {
            r.contains("spacing")
                || r.contains("curly")
                || r.contains("op")
                || r.contains("paren")
                || r.contains("colon")
                || r.contains("comma")
                || r.contains("comment")
        });

        if any_spacing {
            text = fix_all_spacing(&text);
        }

        // Line-based fixes
        if rules.contains("standard:no-trailing-spaces")
            || rules.contains("standard:no-consecutive-blank-lines")
        {
            text = fix_line_based(&text, &rules);
        }

        if text != original {
            std::fs::write(path, text)?;
        }
    }
    Ok(())
}

fn fix_all_spacing(text: &str) -> String {
    let mut t = text.to_string();

    // Pass 1: curly braces — add space before { when preceded by identifier/) ]
    let bytes = t.as_bytes();
    let mut result = String::with_capacity(bytes.len() * 2);
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' && i > 0 {
            let prev = bytes[i - 1];
            if (prev.is_ascii_alphanumeric() || prev == b')' || prev == b']')
                && prev != b'{'
                && prev != b'('
                && prev != b'['
                && prev != b' '
                && prev != b'\n'
            {
                result.push(' ');
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    t = result;

    // Pass 2: comparison operators < >
    // Fix `x>0` → `x > 0` (only when between identifiers/numbers)
    let bytes5 = t.as_bytes();
    let mut result5 = String::new();
    let mut n = 0;
    while n < bytes5.len() {
        let c = bytes5[n];
        if (c == b'<' || c == b'>') && n > 0 && n + 1 < bytes5.len() {
            let prev = bytes5[n - 1];
            let next = bytes5[n + 1];
            // Only fix if surrounded by identifiers/numbers (not generics like <T>)
            if (prev.is_ascii_alphanumeric() || prev == b')')
                && (next.is_ascii_alphanumeric() || next == b'(')
            {
                if prev != b' ' {
                    result5.push(' ');
                }
                result5.push(c as char);
                if next != b' ' {
                    result5.push(' ');
                }
                n += 1;
                continue;
            }
        }
        result5.push(c as char);
        n += 1;
    }
    t = result5;

    // Pass 3: operators — add spaces around = + - * / %
    let ops = [
        "==", "!=", "<=", ">=", "&&", "||", "=", "+", "-", "*", "/", "%", "<", ">",
    ];
    for op in &ops {
        // Case: `x= y` → `x = y`
        let pattern = format!("{} ", op);
        let replacement = format!(" {} ", op);
        // Find patterns like `word= ` → `word = `
        t = t.replace(&format!("{}{} ", pattern, pattern.trim()), &replacement);
    }
    // General fix: add spaces around bare operators (conservative)
    t = t.replace(" =", " =").replace("= ", "= ");
    // Fix `x=1` pattern: letter/number before = without space
    let bytes2 = t.as_bytes();
    let mut result2 = String::new();
    let mut j = 0;
    while j < bytes2.len() {
        let c = bytes2[j];
        result2.push(c as char);
        // After identifier/number, before =, add space if missing
        if c.is_ascii_alphanumeric() && j + 1 < bytes2.len() && bytes2[j + 1] == b'=' {
            if j + 2 < bytes2.len() && bytes2[j + 2] != b'=' && bytes2[j + 2] != b' ' {
                result2.push(' ');
            }
        }
        // After =, before identifier/number, add space if missing
        if c == b'=' && j + 1 < bytes2.len() && bytes2[j + 1].is_ascii_alphanumeric() {
            if j > 0
                && bytes2[j - 1] != b' '
                && bytes2[j - 1] != b'!'
                && bytes2[j - 1] != b'<'
                && bytes2[j - 1] != b'>'
                && bytes2[j - 1] != b'='
            {
                // Don't add if it's inside string literals
                if bytes2[j - 1].is_ascii_alphanumeric() || bytes2[j - 1] == b')' {
                    // Insert space before =
                    let last = result2.pop().unwrap();
                    result2.push(' ');
                    result2.push(last);
                }
            }
            if j + 1 < bytes2.len()
                && bytes2[j + 1] != b' '
                && bytes2[j + 1] != b'"'
                && bytes2[j + 1] != b'\''
            {
                result2.push(' ');
            }
        }
        j += 1;
    }
    t = result2;

    // Pass 3: commas — remove space before, ensure space after
    t = t.replace(" ,", ",").replace(",  ", ", ");

    // Pass 4: parens — remove space after ( and before )
    t = t.replace("( ", "(").replace(" )", ")");

    // Pass 5: colon — ensure space after : in type annotations (not ::)
    let bytes3 = t.as_bytes();
    let mut result3 = String::new();
    let mut k = 0;
    while k < bytes3.len() {
        let c = bytes3[k];
        // Check for bare : (not ::)
        if c == b':' && k > 0 && k + 1 < bytes3.len() {
            if bytes3[k - 1] != b':' && bytes3[k + 1] != b':' {
                // Single colon — ensure space after if followed by identifier
                if bytes3[k + 1].is_ascii_alphanumeric() && bytes3[k + 1] != b' ' {
                    result3.push(':');
                    result3.push(' ');
                    k += 1;
                    continue;
                }
                // Remove space before : in type context (val x : Int → val x: Int)
                if k >= 2 && bytes3[k - 1] == b' ' && bytes3[k - 2].is_ascii_alphanumeric() {
                    result3.pop(); // remove the space before :
                }
            }
        }
        result3.push(c as char);
        k += 1;
    }
    t = result3;

    // Pass 6: comment spacing — add space after //
    let bytes4 = t.as_bytes();
    let mut result4 = String::new();
    let mut m = 0;
    while m < bytes4.len() {
        if m + 2 < bytes4.len()
            && bytes4[m] == b'/'
            && bytes4[m + 1] == b'/'
            && bytes4[m + 2] != b'/'
            && bytes4[m + 2] != b' '
            && bytes4[m + 2] != b'\n'
        {
            result4.push_str("// ");
            m += 2;
        } else {
            result4.push(bytes4[m] as char);
            m += 1;
        }
    }
    t = result4;

    // Pass 7: remove blank lines before }
    while t.contains("\n\n}") {
        t = t.replace("\n\n}", "\n}");
    }

    // Pass 8: merge }\nelse → } else
    t = t
        .replace("}\nelse", "} else")
        .replace("}\n    else", "} else");

    // Pass 9: merge }\ncatch → } catch
    t = t
        .replace("}\ncatch", "} catch")
        .replace("}\n    catch", "} catch");

    // Pass 10: remove double spaces
    while t.contains("  ") {
        t = t.replace("  ", " ");
    }

    // Fix over-corrections: add back newlines
    t = t.replace("\n ", "\n");

    t
}

fn fix_line_based(text: &str, rules: &std::collections::HashSet<&str>) -> String {
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    let trailing_nl = text.ends_with('\n');

    let mut i = 0;
    while i < lines.len() {
        let cur_empty = lines[i].trim().is_empty();
        let prev_empty = if i > 0 {
            lines[i - 1].trim().is_empty()
        } else {
            false
        };

        if rules.contains("standard:no-trailing-spaces") {
            lines[i] = lines[i].trim_end().to_string();
        }
        if rules.contains("standard:no-consecutive-blank-lines") {
            if cur_empty && prev_empty {
                lines.remove(i);
                continue;
            }
        }
        i += 1;
    }

    lines.join("\n") + if trailing_nl { "\n" } else { "" }
}
