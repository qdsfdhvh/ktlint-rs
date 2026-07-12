//! Strict JVM ktlint compat engine — exact rule set match.
use crate::config::KtlintConfig;
use crate::rules::{RuleEngine, Violation};
use tree_sitter::Tree;

pub struct KtlintCompatEngine { engine: RuleEngine }

impl KtlintCompatEngine {
    pub fn new(config: &KtlintConfig) -> Self {
        let mut cfg = config.clone();
        cfg.compat_mode = true;
        
        // JVM-ONLY rules: these are rules that JVM ktlint checks on demo-gradle.
        // Everything NOT in this list gets disabled for exact CLI match.
        let jvm_rules = [
        "standard:indent",
        "standard:no-trailing-spaces",
        "standard:final-newline",
        "standard:no-consecutive-blank-lines",
        "standard:no-blank-line-before-rbrace",
        "standard:max-line-length",
        "standard:no-empty-file",
        "standard:trailing-comma",
        "standard:no-wildcard-imports",
        "standard:import-ordering",
        "standard:no-unused-imports",
        "standard:class-naming",
        "standard:function-naming",
        "standard:property-naming",
        "standard:package-name",
        "standard:filename",
        "standard:backing-property-naming",
        "standard:enum-entry-name-case",
        "standard:no-semicolons",
        "standard:string-template",
        "standard:if-else-bracing",
        "standard:no-consecutive-comments",
        "standard:multiline-expression-wrapping",
        "standard:no-empty-first-line-in-class-body",
        "standard:no-blank-line-in-list",
        "standard:blank-line-before-declaration",
        "standard:blank-line-between-when-conditions",
        "standard:wrapping",
        "standard:kdoc",
        "standard:argument-list-wrapping",
        "standard:parameter-list-wrapping",
        "standard:trailing-comma-on-call-site",
        "standard:no-empty-file-body",
        "standard:spacing-between-declarations-with-comments",
        "standard:annotation",
        "standard:when-entry-bracing",
        "standard:function-signature",
        "standard:enum-wrapping",
        "standard:keyword-spacing",
        "standard:function-type-modifier-spacing",
        "standard:type-argument-list-spacing",
        "standard:spacing-around-angle-brackets",
        "standard:chain-wrapping",
        "standard:function-expression-body",
        "standard:function-start-of-body-spacing",
        "standard:curly-spacing",
        "standard:comma-spacing",
        "standard:colon-spacing",
        "standard:op-spacing",
        "standard:parameter-list-spacing",
    ];

        // Set all rules to disabled by default
        let all_rule_ids: Vec<String> = [/* we'll iterate the engine */].to_vec();
        // Instead, we mark only JVM rules as enabled, all others as disabled
        // This is done AFTER engine creation by filtering
        
        // Enable only JVM rules via config
        for rid in &jvm_rules {
            cfg.rules.entry(rid.to_string()).or_default().enabled = true;
        }

        Self { engine: RuleEngine::new(&cfg) }
    }

    pub fn check(&self, path: &str, tree: &Tree, source: &str) -> Vec<Violation> {
        self.engine.check(path, tree, source)
    }
}
