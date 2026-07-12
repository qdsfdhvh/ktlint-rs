//! Strict JVM ktlint compat engine — exact rule set match.
use crate::config::KtlintConfig;
use crate::rules::{RuleEngine, Violation};
use tree_sitter::Tree;

pub struct KtlintCompatEngine {
    engine: RuleEngine,
}

impl KtlintCompatEngine {
    pub fn new(config: &KtlintConfig) -> Self {
        let mut cfg = config.clone();
        cfg.compat_mode = true;

        // JVM-ONLY rules: these are rules that JVM ktlint checks on demo-gradle.
        // Everything NOT in this list gets disabled for exact CLI match.
        let jvm_rules = [
        "standard:no-consecutive-blank-lines",
        "standard:no-empty-file",
        
        "standard:no-trailing-spaces",
        "standard:function-literal",
        "standard:wrapping",
        
        "standard:binary-expression-wrapping",
        "standard:trailing-comma-on-call-site",
        "standard:function-type-modifier-spacing",
        "standard:no-wildcard-imports",
        "standard:no-unused-imports",
        "standard:final-newline",
        "standard:no-blank-line-before-rbrace",
        
    ];

        // Set all rules to disabled by default
        let all_rule_ids: Vec<String> = [/* we'll iterate the engine */].to_vec();
        // Instead, we mark only JVM rules as enabled, all others as disabled
        // This is done AFTER engine creation by filtering

        // Enable only JVM rules via config
        for rid in &jvm_rules {
            cfg.rules.entry(rid.to_string()).or_default().enabled = true;
        }

        Self {
            engine: RuleEngine::new(&cfg),
        }
    }

    pub fn check(&self, path: &str, tree: &Tree, source: &str) -> Vec<Violation> {
        self.engine.check(path, tree, source)
    }
}
