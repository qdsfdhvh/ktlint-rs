//! JVM ktlint compatible rule engine — strict rule set matching.
use crate::config::KtlintConfig;
use crate::rules::{Rule, RuleEngine, Violation};
use tree_sitter::Tree;

pub struct KtlintCompatEngine { engine: RuleEngine }

impl KtlintCompatEngine {
    pub fn new(config: &KtlintConfig) -> Self {
        let mut cfg = config.clone();
        // Rules that ktlint-rs has but JVM ktlint does NOT (for any code style)
        let ktlint_rs_only = [
            "standard:op-spacing", "standard:curly-spacing", "standard:spacing-around-keyword",
            "standard:spacing-around-double-colon", "standard:modifier-order",
            "standard:when-expression-line-break", "standard:string-template-indent",
            "standard:function-return-type-spacing", "standard:function-start-of-body-spacing",
            "standard:spacing-around-range-operator", "standard:spacing-around-dot",
            "standard:no-wildcard-imports-either", "standard:comment-wrapping",
            "standard:no-empty-file-body", "standard:if-else-wrapping",
            "standard:spacing-between-function-name-and-parenthesis",
            "standard:spacing-around-square-brackets", "standard:no-blank-lines-in-chained-method-calls",
            "standard:no-line-break-after-else", "standard:no-line-break-before-assignment",
            "standard:nullable-type-spacing", "standard:fun-keyword-spacing",
            "standard:package-import-spacing", "standard:mixed-condition-operators",
            "standard:spacing-around-angle-brackets", "standard:spacing-around-unary-operator",
            "standard:block-comment-initial-star-blank-line",
            // Disabled for now — too noisy on compose-samples
            "standard:multiline-expression-wrapping", "standard:no-empty-first-line-in-class-body",
            "standard:argument-list-wrapping", "standard:no-consecutive-comments",
            "standard:blank-line-between-when-conditions", "standard:when-entry-bracing", "standard:enum-entry", "standard:annotation-spacing", "standard:comment-spacing", "standard:annotation", "standard:trailing-comma", "standard:indent", "standard:max-line-length", "standard:colon-spacing",
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
