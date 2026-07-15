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
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_undoc(
            tree.root_node(),
            &mut v,
            "class_declaration",
            "UndocumentedPublicClass",
            "Public class is missing KDoc",
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
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_undoc(
            tree.root_node(),
            &mut v,
            "function_declaration",
            "UndocumentedPublicFunction",
            "Public function is missing KDoc",
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
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_undoc(
            tree.root_node(),
            &mut v,
            "property_declaration",
            "UndocumentedPublicProperty",
            "Public property is missing KDoc",
        );
        v
    }
}

fn walk_undoc(
    n: tree_sitter::Node,
    v: &mut Vec<Violation>,
    target_kind: &str,
    rule_name: &str,
    msg: &str,
) {
    if n.kind() == target_kind {
        let pos = n.start_position();
        let mut has_kdoc = false;
        // Check if previous sibling or nearby node is a KDoc comment
        if let Some(parent) = n.parent() {
            let target_id = n.id();
            for i in 0..parent.child_count() {
                if let Some(child) = parent.child(i) {
                    if child.id() == target_id {
                        break;
                    }
                    let ck = child.kind();
                    if ck == "comment" || ck == "multiline_comment" || ck == "block_comment" {
                        // Simple heuristic: comment ends with */ is KDoc
                        let _range = child.byte_range();
                        // We can't easily get text from Node, so use a position heuristic
                        let child_end = child.end_position().row;
                        let our_start = pos.row;
                        if child_end + 1 == our_start || child_end == our_start {
                            has_kdoc = true;
                            break;
                        }
                    }
                }
            }
        }
        if !has_kdoc {
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
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            walk_undoc(c, v, target_kind, rule_name, msg);
        }
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
