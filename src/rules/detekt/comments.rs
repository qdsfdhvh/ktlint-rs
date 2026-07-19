//! detekt comments rules — documentation quality checks. L0, text-level.

use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

// ── DeprecatedBlockTag ──
pub struct DeprecatedBlockTag;
impl Rule for DeprecatedBlockTag {
    fn id(&self) -> &'static str {
        "detekt:comments:DeprecatedBlockTag"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_kdoc = false;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("/**") {
                in_kdoc = true;
                continue;
            }
            if in_kdoc {
                if t.ends_with("*/") {
                    in_kdoc = false;
                    continue;
                }
                let stripped = t.trim_start_matches('*').trim();
                if stripped == "@deprecated" && !stripped.contains(' ') {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:comments:DeprecatedBlockTag".into(),
                        message: "@deprecated should include a description".into(),
                        auto_fixable: false,
                    });
                }
            }
        }
        v
    }
}

// ── EndOfSentenceFormat ──
pub struct EndOfSentenceFormat;
impl Rule for EndOfSentenceFormat {
    fn id(&self) -> &'static str {
        "detekt:comments:EndOfSentenceFormat"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_kdoc = false;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("/**") {
                in_kdoc = true;
                continue;
            }
            if in_kdoc && t.ends_with("*/") {
                in_kdoc = false;
                continue;
            }
            if !in_kdoc {
                continue;
            }
            let stripped = t.trim_start_matches('*').trim();
            if stripped.is_empty() || stripped.starts_with('@') {
                continue;
            }
            if stripped.starts_with("```") || stripped.starts_with('[') {
                continue;
            }
            let last_char = stripped.chars().last().unwrap_or('.');
            if last_char.is_lowercase() && last_char != '.' {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: "detekt:comments:EndOfSentenceFormat".into(),
                    message: "KDoc sentence should end with a period".into(),
                    auto_fixable: false,
                });
            }
        }
        v
    }
}

// ── AbsentOrWrongFileLicense ──
pub struct AbsentOrWrongFileLicense;
impl Rule for AbsentOrWrongFileLicense {
    fn id(&self) -> &'static str {
        "detekt:comments:AbsentOrWrongFileLicense"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let first = source.lines().next().unwrap_or("").trim();
        let has_license = !first.is_empty()
            && (first.starts_with("/*") || first.starts_with("//"))
            && (first.to_lowercase().contains("copyright")
                || first.to_lowercase().contains("license")
                || first.to_lowercase().contains("apache")
                || first.to_lowercase().contains("mit"));
        if !has_license {
            return vec![Violation {
                file: String::new(),
                line: 1,
                col: 1,
                rule_id: "detekt:comments:AbsentOrWrongFileLicense".into(),
                message: "File header should include a license notice".into(),
                auto_fixable: false,
            }];
        }
        Vec::new()
    }
}

// ── DeprecatedAnnotation ──
pub struct DeprecatedAnnotation;
impl Rule for DeprecatedAnnotation {
    fn id(&self) -> &'static str {
        "detekt:comments:DeprecatedAnnotation"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, line)| {
                let t = line.trim();
                if t.contains("@Deprecated")
                    && !t.contains("message")
                    && !t.contains("replaceWith")
                    && !t.contains("\"")
                {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:comments:DeprecatedAnnotation".into(),
                        message: "@Deprecated should include message or replaceWith".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── KDocMissing ──
pub struct KDocMissing;
impl Rule for KDocMissing {
    fn id(&self) -> &'static str {
        "detekt:comments:KDocMissing"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut prev_was_kdoc = false;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("/**") || t.starts_with(" *") || t.starts_with(" */") || t == "*/" {
                prev_was_kdoc = true;
                continue;
            }
            if t.starts_with("public ")
                || t.starts_with("open ")
                || t.starts_with("internal ")
                || t.starts_with("protected ")
            {
                if !prev_was_kdoc {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:comments:KDocMissing".into(),
                        message: "Public/internal declaration is missing KDoc".into(),
                        auto_fixable: false,
                    });
                }
            }
            prev_was_kdoc = false;
        }
        v
    }
}

// ── NonAsciiCharacters ──
pub struct NonAsciiCharacters;
impl Rule for NonAsciiCharacters {
    fn id(&self) -> &'static str {
        "detekt:comments:NonAsciiCharacters"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_comment = false;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("/**") || t.starts_with("/*") {
                in_comment = true;
            }
            if in_comment {
                if t.ends_with("*/") {
                    in_comment = false;
                }
                for (col, c) in t.char_indices() {
                    if c as u32 > 127 && !c.is_whitespace() {
                        v.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: col + 1,
                            rule_id: "detekt:comments:NonAsciiCharacters".into(),
                            message: format!("Non-ASCII character '{}' in comment", c),
                            auto_fixable: false,
                        });
                        break;
                    }
                }
            }
        }
        v
    }
}

// ── UndocumentedPublicClass ──
pub struct UndocumentedPublicClass;
impl Rule for UndocumentedPublicClass {
    fn id(&self) -> &'static str {
        "detekt:comments:UndocumentedPublicClass"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        walk_undocumented_shared(
            tree.root_node(),
            &mut [&mut v, &mut Vec::new(), &mut Vec::new()],
            None,
            bytes,
        );
        v
    }
}

// ── UndocumentedPublicFunction ──
pub struct UndocumentedPublicFunction;
impl Rule for UndocumentedPublicFunction {
    fn id(&self) -> &'static str {
        "detekt:comments:UndocumentedPublicFunction"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        walk_undocumented_shared(
            tree.root_node(),
            &mut [&mut Vec::new(), &mut v, &mut Vec::new()],
            None,
            bytes,
        );
        v
    }
}

// ── UndocumentedPublicProperty ──
pub struct UndocumentedPublicProperty;
impl Rule for UndocumentedPublicProperty {
    fn id(&self) -> &'static str {
        "detekt:comments:UndocumentedPublicProperty"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        walk_undocumented_shared(
            tree.root_node(),
            &mut [&mut Vec::new(), &mut Vec::new(), &mut v],
            None,
            bytes,
        );
        v
    }
}

/// Single-pass CST traversal for all three UndocumentedPublic* rules,
/// carrying the last comment's end row as state — no Node::parent() calls.
fn walk_undocumented_shared(
    n: tree_sitter::Node,
    v: &mut [&mut Vec<Violation>; 3],
    last_comment_end: Option<usize>,
    bytes: &[u8],
) -> Option<usize> {
    let mut next_last = last_comment_end;

    // Update `next_last` if this node is a comment
    let is_comment = matches!(n.kind(), "multiline_comment" | "block_comment" | "comment");
    if is_comment {
        next_last = Some(n.end_position().row);
    }

    // Check declarations for missing KDoc
    match n.kind() {
        "class_declaration" => check_undoc(
            bytes,
            &n,
            &mut v[0],
            last_comment_end,
            "UndocumentedPublicClass",
            "Public class is missing KDoc",
        ),
        "function_declaration" => check_undoc(
            bytes,
            &n,
            &mut v[1],
            last_comment_end,
            "UndocumentedPublicFunction",
            "Public function is missing KDoc",
        ),
        "property_declaration" => check_undoc(
            bytes,
            &n,
            &mut v[2],
            last_comment_end,
            "UndocumentedPublicProperty",
            "Public property is missing KDoc",
        ),
        _ => {}
    }

    // Recurse into children in order; last comment end propagates forward
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            next_last = walk_undocumented_shared(c, v, next_last, bytes);
        }
    }
    next_last
}

fn check_undoc(
    bytes: &[u8],
    n: &tree_sitter::Node,
    v: &mut Vec<Violation>,
    prev_comment_end: Option<usize>,
    rule_name: &str,
    msg: &str,
) {
    let has_kdoc = prev_comment_end.is_some_and(|end| {
        let start = n.start_position().row;
        end + 1 == start || end == start
    });
    // Skip private, inner, override declarations (detekt-compatible)
    let node_text = n.utf8_text(bytes).unwrap_or("");
    let is_non_public = node_text
        .split_whitespace()
        .any(|w| w == "private" || w == "inner" || w == "override");
    if !has_kdoc && !is_non_public {
        let pos = n.start_position();
        v.push(Violation {
            file: String::new(),
            line: pos.row + 1,
            col: pos.column + 1,
            rule_id: format!("detekt:comments:{}", rule_name),
            message: msg.into(),
            auto_fixable: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(r: &dyn Rule, s: &str) -> Vec<Violation> {
        r.check(&KotlinParser::new().parse(s), s)
    }

    #[test]
    fn deprecated_with_desc_ok() {
        assert!(c(
            &DeprecatedBlockTag,
            "/**\n * @deprecated Use newFoo instead\n */\nfun foo()\n"
        )
        .is_empty());
    }
    #[test]
    fn deprecated_no_desc_bad() {
        assert!(!c(&DeprecatedBlockTag, "/**\n * @deprecated\n */\nfun foo()\n").is_empty());
    }

    #[test]
    fn sentence_ok() {
        assert!(c(
            &EndOfSentenceFormat,
            "/**\n * This is a sentence.\n */\nfun foo()\n"
        )
        .is_empty());
    }
    #[test]
    fn sentence_bad() {
        assert!(!c(
            &EndOfSentenceFormat,
            "/**\n * this is a sentence\n */\nfun foo()\n"
        )
        .is_empty());
    }

    #[test]
    fn license_exists() {
        assert!(c(
            &AbsentOrWrongFileLicense,
            "/* Copyright 2024 */\nfun foo()\n"
        )
        .is_empty());
    }
    #[test]
    fn license_missing() {
        assert!(!c(&AbsentOrWrongFileLicense, "fun foo()\n").is_empty());
    }

    #[test]
    fn deprecated_annot_ok() {
        assert!(c(
            &DeprecatedAnnotation,
            "@Deprecated(message = \"use newFoo\")\nfun foo()\n"
        )
        .is_empty());
    }
    #[test]
    fn deprecated_annot_bad() {
        assert!(!c(&DeprecatedAnnotation, "@Deprecated\nfun foo()\n").is_empty());
    }

    #[test]
    fn kdoc_missing_bad() {
        assert!(!c(&KDocMissing, "public class Foo\n").is_empty());
    }
    #[test]
    fn kdoc_present_ok() {
        assert!(c(&KDocMissing, "/** doc */\npublic class Foo\n").is_empty());
    }

    #[test]
    fn nonascii_ok() {
        assert!(c(&NonAsciiCharacters, "// hello\nfun foo()\n").is_empty());
    }

    #[test]
    fn undoc_class_ok() {
        assert!(c(&UndocumentedPublicClass, "/** KDoc */\nclass Foo\n").is_empty());
    }
    #[test]
    fn undoc_class_bad() {
        assert!(!c(&UndocumentedPublicClass, "class Foo\n").is_empty());
    }

    #[test]
    fn undoc_fn_bad() {
        assert!(!c(&UndocumentedPublicFunction, "fun foo() { }\n").is_empty());
    }
}
