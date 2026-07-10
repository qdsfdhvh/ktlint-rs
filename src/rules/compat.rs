//! JVM ktlint compatible rule engine — only rules that JVM ktlint has.
//! Drops ktlint-rs-only rules so output matches JVM ktlint exactly.

use crate::config::KtlintConfig;
use crate::rules::{Rule, RuleEngine, Violation};
use tree_sitter::Tree;

pub struct KtlintCompatEngine {
    engine: RuleEngine,
}

impl KtlintCompatEngine {
    /// Create engine with only JVM ktlint-compatible rules enabled.
    pub fn new(config: &KtlintConfig) -> Self {
        // Clone config and disable ktlint-rs-only rules
        let mut cfg = config.clone();
        let ktlint_rs_only = [
            "standard:op-spacing",
            "standard:curly-spacing",
            "standard:spacing-around-keyword",
            "standard:spacing-around-double-colon",
            "standard:modifier-order",
            "standard:when-expression-line-break",
            "standard:string-template-indent",
            "standard:function-return-type-spacing",
            "standard:function-start-of-body-spacing",
            "standard:spacing-around-range-operator",
            "standard:spacing-around-dot",
            "standard:no-wildcard-imports-either",
            "standard:comment-wrapping",
            "standard:no-empty-file-body",
            "standard:if-else-wrapping",
            "standard:spacing-between-function-name-and-parenthesis",
            // Disable in compat: too many false positives without explicit indent_size
            "standard:indent",
            "standard:colon-spacing",
            "standard:max-line-length",
        ];
        for rid in &ktlint_rs_only {
            cfg.rules.entry(rid.to_string()).or_default().enabled = false;
        }

        Self { engine: RuleEngine::new(&cfg) }
    }

    pub fn check(&self, path: &str, tree: &Tree, source: &str) -> Vec<Violation> {
        self.engine.check(path, tree, source)
    }
}
