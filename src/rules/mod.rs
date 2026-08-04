//! ktlint rule engine — linting rules for Kotlin code.

use crate::config::{CodeStyle, KtlintConfig};
use crate::resolver::builder::build_symbol_table;
use crate::resolver::SymbolTable;
use tree_sitter::Tree;

pub mod builtins;
pub mod detekt;
pub mod imports;
pub mod naming;
pub mod new_rules;
pub mod new_rules2;
pub mod new_rules3;
pub mod new_rules4;
pub mod phase1_more;
pub mod phase3_rules;
pub mod phase3b_rules;
pub mod registry;
pub mod spacing;
pub mod structure;
pub mod suppress;
pub mod wrapping;
// pub use phase3b_rules::*; // re-exported by individual rules
pub mod final_rules;
// pub use final_rules::*; // re-exported by individual rules

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
    fn requires_type_resolution(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation>;

    /// Lint with a pre-built SymbolTable. L1 rules override; others delegate to `check`.
    fn check_with_symbols(
        &self,
        tree: &Tree,
        source: &str,
        _sym: Option<&SymbolTable>,
    ) -> Vec<Violation> {
        self.check(tree, source)
    }
}

const OFFICIAL_CODE_STYLE_ONLY: &[&str] = &[
    "standard:blank-line-before-declaration",
    "standard:chain-method-continuation",
    "standard:if-else-bracing",
    "standard:if-else-wrapping",
    "standard:multiline-expression-wrapping",
    "standard:no-blank-line-in-list",
    "standard:no-consecutive-comments",
    "standard:no-empty-first-line-in-class-body",
    "standard:no-single-line-block-comment",
    "standard:string-template-indent",
    "standard:try-catch-finally-spacing",
    "standard:when-entry-bracing",
];

pub(crate) fn code_style_allows(rule_id: &str, code_style: CodeStyle) -> bool {
    code_style == CodeStyle::KtlintOfficial || !OFFICIAL_CODE_STYLE_ONLY.contains(&rule_id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleMetadata {
    pub id: &'static str,
    pub auto_fixable: bool,
    pub requires_type_resolution: bool,
    pub enabled_by_ruleset: bool,
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

    pub fn inventory(config: &KtlintConfig) -> Vec<RuleMetadata> {
        let engine = Self::new(config);
        let mut inventory: Vec<_> = engine
            .rules
            .iter()
            .map(|rule| RuleMetadata {
                id: rule.id(),
                auto_fixable: rule.auto_fixable(),
                requires_type_resolution: rule.requires_type_resolution(),
                enabled_by_ruleset: engine.rule_set_allows(rule.id()),
            })
            .collect();
        inventory.sort_by_key(|rule| rule.id);
        inventory
    }

    pub fn check(&self, path: &str, tree: &Tree, source: &str) -> Vec<Violation> {
        // Build SymbolTable once per file — 11 L1 rules share it.
        let sym_table = build_symbol_table(source, tree.root_node());

        let mut violations = Vec::new();
        for rule in &self.rules {
            if !self.config.is_rule_enabled(rule.id()) {
                continue;
            }
            if !code_style_allows(rule.id(), self.config.code_style) {
                continue;
            }
            if !self.rule_set_allows(rule.id()) {
                continue;
            }
            if rule.requires_type_resolution() && self.config.skip_type_resolution {
                continue;
            }
            for mut v in rule.check_with_symbols(tree, source, Some(&sym_table)) {
                v.file = path.to_string();
                violations.push(v);
            }
        }
        // Text-scanning spacing rules can report inside comments/KDoc (e.g.
        // `https://` in a doc link, `*` in a license header). Filter those out
        // — comment content is not code. Max-line-length etc. still report.
        let mut comment_rows = Self::comment_rows(tree.root_node());
        Self::comment_rows_in_source(source, &mut comment_rows);
        let spacing_ids: &[&str] = &[
            "standard:colon-spacing",
            "standard:op-spacing",
            "standard:curly-spacing",
            "standard:comma-spacing",
            "standard:dot-spacing",
            "standard:paren-spacing",
            "standard:keyword-spacing",
            "standard:range-spacing",
            "standard:modifier-order",
            "standard:spacing-around-angle-brackets",
            "standard:type-argument-list-spacing",
            "standard:type-parameter-list-spacing",
            "standard:annotation-spacing",
            "standard:function-return-type-spacing",
            "standard:no-semi",
            "standard:double-colon-spacing",
            "standard:value-parameter-comment",
            "standard:value-argument-comment",
            "standard:type-parameter-comment",
            "standard:type-argument-comment",
            "standard:function-naming",
            "standard:enum-entry-name-case",
            "standard:no-unused-imports",
        ];
        violations.retain(|v| {
            !(comment_rows.contains(&(v.line.saturating_sub(1)))
                && spacing_ids.contains(&v.rule_id.as_str()))
        });
        violations
    }

    fn comment_rows(node: tree_sitter::Node) -> std::collections::HashSet<usize> {
        let mut rows = std::collections::HashSet::new();
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            if current.kind().contains("comment") {
                for row in current.start_position().row..=current.end_position().row {
                    rows.insert(row);
                }
            }
            for i in (0..current.child_count()).rev() {
                if let Some(child) = current.child(i) {
                    stack.push(child);
                }
            }
        }
        rows
    }

    /// Text-scanning fallback: some KDoc/comment lines (`/**`, `*/`, `*`) are
    /// not covered by the CST comment node's row span, so also mark them by
    /// their leading tokens.
    fn comment_rows_in_source(source: &str, rows: &mut std::collections::HashSet<usize>) {
        for (i, line) in source.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//")
                || t.starts_with("/*")
                || t.starts_with("*/")
                || t.starts_with("*")
                || t.starts_with("/**")
            {
                rows.insert(i);
            }
        }
    }

    /// Rs-only rules: no JVM ktlint equivalent. Excluded from default --ruleset ktlint
    /// unless --compat is passed. Source: docs/RULE_PLAN.md Part 2.
    const RS_ONLY: &[&str] = &[
        "standard:ij_kotlin_allow_trailing_comma",
        "standard:kdoc-no-empty-first-line",
        "standard:no-trailing-spaces-in-kdoc",
        "standard:no-unnecessary-parentheses-before-trailing-lambda",
        "standard:no-empty-line-after-kdoc",
        "standard:no-blank-line-before-list-closing",
        "standard:no-empty-file-body",
        "standard:no-single-expression-body",
        "standard:no-trailing-spaces-in-string-template",
        "standard:no-wildcard-imports-either",
        "standard:spacing-between-declarations",
        "standard:trailing-comma",
        "standard:no-trailing-spaces-in-block-comment",
        "standard:try-catch-finally-wrapping",
        "standard:when-expression-line-break",
    ];

    fn rule_set_allows(&self, rule_id: &str) -> bool {
        match self.config.rule_set {
            crate::config::RuleSet::Both => true,
            crate::config::RuleSet::DetektOnly => rule_id.starts_with("detekt:"),
            crate::config::RuleSet::KtlintOnly => {
                if !rule_id.starts_with("detekt:") {
                    // Exclude rs-only rules if compat mode is off
                    self.config.compat || !Self::RS_ONLY.contains(&rule_id)
                } else {
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod rule_set_tests {
    use super::*;
    use crate::config::{KtlintConfig, RuleSet};

    fn config_with(ruleset: RuleSet, compat: bool) -> KtlintConfig {
        KtlintConfig {
            rule_set: ruleset,
            compat,
            ..KtlintConfig::default()
        }
    }

    #[test]
    fn t50_ktlint_excludes_rs_only() {
        let engine = RuleEngine::new(&config_with(RuleSet::KtlintOnly, false));
        assert!(engine.rule_set_allows("standard:curly-spacing"));
        assert!(!engine.rule_set_allows("standard:no-single-expression-body"));
        assert!(!engine.rule_set_allows("standard:spacing-between-declarations"));
        assert!(!engine.rule_set_allows("detekt:style:VarCouldBeVal"));
    }

    #[test]
    fn t50_compat_enables_rs_only() {
        let engine = RuleEngine::new(&config_with(RuleSet::KtlintOnly, true));
        assert!(engine.rule_set_allows("standard:no-single-expression-body"));
        assert!(engine.rule_set_allows("standard:spacing-between-declarations"));
    }

    #[test]
    fn t50_detekt_only_excludes_standard() {
        let engine = RuleEngine::new(&config_with(RuleSet::DetektOnly, false));
        assert!(!engine.rule_set_allows("standard:curly-spacing"));
        assert!(!engine.rule_set_allows("standard:no-single-expression-body"));
        assert!(engine.rule_set_allows("detekt:style:VarCouldBeVal"));
    }

    #[test]
    fn t50_both_includes_all() {
        let engine = RuleEngine::new(&config_with(RuleSet::Both, false));
        assert!(engine.rule_set_allows("standard:no-single-expression-body"));
        assert!(engine.rule_set_allows("detekt:style:VarCouldBeVal"));
    }
}

pub use builtins::*;
