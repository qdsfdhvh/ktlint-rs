//! ktlint rule engine — linting rules for Kotlin code.

use crate::config::KtlintConfig;
use tree_sitter::Tree;

pub mod detekt;
pub mod registry;
pub mod imports;
pub mod naming;
pub mod new_rules;
pub mod new_rules2;
pub mod new_rules3;
pub mod new_rules4;
pub mod phase1_more;
pub mod phase1_rules;
pub mod phase3b_rules;
pub mod spacing;
pub mod structure;
pub mod suppress;
pub mod wrapping;
pub use phase3b_rules::*;
pub mod final_rules;
pub use final_rules::*;

#[derive(Debug, Clone)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub rule_id: String,
    pub message: String,
    pub auto_fixable: bool,
}

pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn auto_fixable(&self) -> bool {
        true
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation>;
}

pub struct RuleEngine {
    config: KtlintConfig,
    rules: Vec<Box<dyn Rule>>,
}

impl RuleEngine {
    pub fn new(config: &KtlintConfig) -> Self {
        let rules: Vec<Box<dyn Rule>> = registry::Registry::all_rules(config);
        Self {
            config: config.clone(),
            rules,
        }
    }

    pub fn check(&self, path: &str, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        for rule in &self.rules {
            if !self.config.is_rule_enabled(rule.id()) {
                continue;
            }
            if !self.rule_set_allows(rule.id()) {
                continue;
            }
            for mut v in rule.check(tree, source) {
                v.file = path.to_string();
                violations.push(v);
            }
        }
        violations
    }

    fn rule_set_allows(&self, rule_id: &str) -> bool {
        match self.config.rule_set {
            crate::config::RuleSet::Both => true,
            crate::config::RuleSet::DetektOnly => rule_id.starts_with("detekt:"),
            crate::config::RuleSet::KtlintOnly => !rule_id.starts_with("detekt:"),
        }
    }
}
// ── Built-in simple rules ────────────────────────────────────────────

pub struct NoTrailingSpaces;
impl Rule for NoTrailingSpaces {
    fn id(&self) -> &'static str {
        "standard:no-trailing-spaces"
    }
    fn check(&self, _: &Tree, s: &str) -> Vec<Violation> {
        s.lines()
            .enumerate()
            .filter(|(_, l)| l.ends_with(' ') || l.ends_with('\t'))
            .map(|(i, _)| Violation {
                file: String::new(),
                line: i + 1,
                col: 0,
                rule_id: self.id().into(),
                message: "Trailing space(s)".into(),
                auto_fixable: true,
            })
            .collect()
    }
}
pub struct FinalNewline;
impl Rule for FinalNewline {
    fn id(&self) -> &'static str {
        "standard:final-newline"
    }
    fn check(&self, _: &Tree, s: &str) -> Vec<Violation> {
        if s.is_empty() || s.ends_with('\n') {
            vec![]
        } else {
            vec![Violation {
                file: String::new(),
                line: s.lines().count(),
                col: 0,
                rule_id: self.id().into(),
                message: "File must end with a newline".into(),
                auto_fixable: true,
            }]
        }
    }
}
pub struct NoConsecutiveBlankLines;
impl Rule for NoConsecutiveBlankLines {
    fn id(&self) -> &'static str {
        "standard:no-consecutive-blank-lines"
    }
    fn check(&self, _: &Tree, s: &str) -> Vec<Violation> {
        let mut v = vec![];
        let mut b = 0;
        for (i, l) in s.lines().enumerate() {
            if l.trim().is_empty() {
                b += 1;
                if b > 1 {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 0,
                        rule_id: self.id().into(),
                        message: "Needless blank line(s)".into(),
                        auto_fixable: true,
                    });
                }
            } else {
                b = 0;
            }
        }
        v
    }
}
pub struct NoWildcardImports;
impl Rule for NoWildcardImports {
    fn id(&self) -> &'static str {
        "standard:no-wildcard-imports"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _: &Tree, s: &str) -> Vec<Violation> {
        s.lines()
            .enumerate()
            .filter(|(_, l)| {
                let t = l.trim();
                t.starts_with("import ") && t.contains(".*")
            })
            .map(|(i, _)| Violation {
                file: String::new(),
                line: i + 1,
                col: 0,
                rule_id: self.id().into(),
                message: "Wildcard import".into(),
                auto_fixable: false,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_trailing_spaces_detects() {
        let rule = NoTrailingSpaces;
        let violations = rule.check(
            &crate::parser::KotlinParser::new().parse("fun test() \n"),
            "fun test() \n",
        );
        assert!(!violations.is_empty());
    }

    #[test]
    fn final_newline_missing() {
        let rule = FinalNewline;
        let v = rule.check(
            &crate::parser::KotlinParser::new().parse("fun a() {}"),
            "fun a() {}",
        );
        assert!(!v.is_empty());
    }
}
