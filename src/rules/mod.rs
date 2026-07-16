//! ktlint rule engine — linting rules for Kotlin code.

use crate::config::KtlintConfig;
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
pub mod phase3b_rules;
pub mod registry;
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
    /// Rules that need Kotlin type resolution (L2). Skipped when --skip-type-resolution is set.
    fn requires_type_resolution(&self) -> bool {
        false
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
            // Skip rules that need type resolution when flag is set
            if rule.requires_type_resolution() && self.config.skip_type_resolution {
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
// Built-in rules (extracted to builtins.rs)
pub use builtins::*;
