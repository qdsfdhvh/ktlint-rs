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
            "standard:spacing-around-keyword",
            "standard:spacing-around-double-colon",
            "standard:modifier-order",
            "standard:when-expression-line-break",
            "standard:string-template-indent",
            "standard:spacing-around-range-operator",
            "standard:spacing-around-dot",
            "standard:no-wildcard-imports-either",
            "standard:no-empty-file-body",
            "standard:spacing-between-function-name-and-parenthesis",
            "standard:spacing-around-square-brackets",
            "standard:no-blank-lines-in-chained-method-calls",
            "standard:no-line-break-after-else",
            "standard:no-line-break-before-assignment",
            "standard:nullable-type-spacing",
            "standard:fun-keyword-spacing",
            "standard:package-import-spacing",
            "standard:mixed-condition-operators",
            "standard:spacing-around-angle-brackets",
            "standard:spacing-around-unary-operator",
            "standard:if-else-wrapping",
            "standard:comment-wrapping",
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
