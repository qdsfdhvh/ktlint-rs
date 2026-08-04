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

        // Detect KTS files: if no class/fun/object declarations, skip indent
        let is_kts = !lines.iter().any(|l| {
            let t = l.trim();
            let kw = t.split_whitespace().next().unwrap_or("");
            matches!(
                kw,
                "class" | "fun" | "object" | "interface" | "enum" | "data"
            ) && !t.starts_with("//")
                && !t.starts_with("/*")
                && !t.starts_with("*")
        });
        if is_kts {
            return violations;
        }

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let spaces = line.len() - trimmed.len();

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

            // Core JVM logic: indent must be a multiple of indent_size.
            let mut too_shallow = false;
            if spaces % is != 0 {
                too_shallow = true;
            }
            // Also flag code that sits at the top level of a block but has no
            // indentation at all (`class Foo {\nval x`): clearly unformatted
            // new code that the formatter should fix. Closing braces and
            // continuation lines are skipped.
            if !too_shallow
                && spaces == 0
                && !trimmed.starts_with('}')
                && expected_depth(&lines, i) > 0
            {
                too_shallow = true;
            }
            if too_shallow {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: format!(
                        "Unexpected indentation ({}) (should be multiple of {})",
                        spaces, is
                    ),
                    auto_fixable: true,
                });
            }
        }
        violations
    }
}

/// Brace depth before a given line — used to detect zero-indent lines inside
/// a block without full continuation analysis.
fn expected_depth(lines: &[&str], target: usize) -> usize {
    let mut depth = 0usize;
    for (i, line) in lines.iter().enumerate() {
        if i >= target {
            break;
        }
        let t = line.trim();
        // Skip string content: `{`/`}` inside quotes (or raw strings) are not
        // block delimiters and would inflate the depth.
        let mut in_string = false;
        let mut cleaned = String::with_capacity(t.len());
        let mut chars = t.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '"' {
                in_string = !in_string;
            }
            if in_string && (c == '{' || c == '}') {
                continue;
            }
            cleaned.push(c);
        }
        let opens = cleaned.bytes().filter(|b| *b == b'{').count();
        let closes = cleaned.bytes().filter(|b| *b == b'}').count();
        depth = depth.saturating_add(opens).saturating_sub(closes);
    }
    depth
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
        let src = "fun f(x: Int,\n        y: String) {\n}\n";
        assert!(check(src, 4).is_empty());
    }

    // Ignored — KTS file with no class/fun declarations
    #[test]
    fn kts_ignored() {
        let src = "plugins {\n    id(\"com.android\")\n}\n";
        assert!(check(src, 4).is_empty());
    }
}
