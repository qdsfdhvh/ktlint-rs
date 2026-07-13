//! detekt comments rules — documentation quality checks. L0, text-level.

use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

// ── DeprecatedBlockTag ──
pub struct DeprecatedBlockTag;
impl Rule for DeprecatedBlockTag {
    fn id(&self) -> &'static str { "detekt:comments:DeprecatedBlockTag" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_kdoc = false;
        let mut kdoc_start = 0;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("/**") {
                in_kdoc = true;
                kdoc_start = i;
                continue;
            }
            if in_kdoc {
                if t.ends_with("*/") {
                    in_kdoc = false;
                    continue;
                }
                // Check for @deprecated without description
                let stripped = t.trim_start_matches('*').trim();
                if stripped == "@deprecated" && !stripped.contains(' ') {
                    v.push(Violation {
                        file: String::new(), line: i + 1, col: 1,
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
    fn id(&self) -> &'static str { "detekt:comments:EndOfSentenceFormat" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_kdoc = false;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("/**") { in_kdoc = true; continue; }
            if in_kdoc && t.ends_with("*/") { in_kdoc = false; continue; }
            if !in_kdoc { continue; }
            // Check KDoc line content: sentences should end with period
            let stripped = t.trim_start_matches('*').trim();
            if stripped.is_empty() || stripped.starts_with('@') { continue; }
            // Skip code blocks, links
            if stripped.starts_with("```") || stripped.starts_with('[') { continue; }
            // If the line ends with a lowercase letter and no period, flag it
            let last_char = stripped.chars().last().unwrap_or('.');
            if last_char.is_lowercase() && last_char != '.' {
                v.push(Violation {
                    file: String::new(), line: i + 1, col: 1,
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
    fn id(&self) -> &'static str { "detekt:comments:AbsentOrWrongFileLicense" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        // Check if the file starts with a license/copyright header
        let first = source.lines().next().unwrap_or("").trim();
        let has_license = !first.is_empty() &&
            (first.starts_with("/*") || first.starts_with("//") || first.starts_with("/*"))
            && (first.to_lowercase().contains("copyright")
                || first.to_lowercase().contains("license")
                || first.to_lowercase().contains("apache")
                || first.to_lowercase().contains("mit"));
        if !has_license {
            return vec![Violation {
                file: String::new(), line: 1, col: 1,
                rule_id: "detekt:comments:AbsentOrWrongFileLicense".into(),
                message: "File header should include a license notice".into(),
                auto_fixable: false,
            }];
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(r: &dyn Rule, s: &str) -> Vec<Violation> { r.check(&KotlinParser::new().parse(s), s) }

    #[test] fn deprecated_with_desc_ok() {
        assert!(c(&DeprecatedBlockTag, "/**\n * @deprecated Use newFoo instead\n */\nfun foo()\n").is_empty());
    }
    #[test] fn deprecated_no_desc_bad() {
        assert!(!c(&DeprecatedBlockTag, "/**\n * @deprecated\n */\nfun foo()\n").is_empty());
    }

    #[test] fn sentence_ok() {
        assert!(c(&EndOfSentenceFormat, "/**\n * This is a sentence.\n */\nfun foo()\n").is_empty());
    }
    #[test] fn sentence_bad() {
        assert!(!c(&EndOfSentenceFormat, "/**\n * this is a sentence\n */\nfun foo()\n").is_empty());
    }

    #[test] fn license_exists() {
        assert!(c(&AbsentOrWrongFileLicense, "/* Copyright 2024 */\nfun foo()\n").is_empty());
    }
    #[test] fn license_missing() {
        assert!(!c(&AbsentOrWrongFileLicense, "fun foo()\n").is_empty());
    }
}
