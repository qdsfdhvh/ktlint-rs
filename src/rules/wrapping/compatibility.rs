//! Deprecated ktlint 1.8 wrapping providers retained as no-op compatibility IDs.

use crate::rules::{Rule, Violation};

/// Deprecated since ktlint 1.7. All behavior moved to
/// `standard:expression-operand-wrapping`.
pub struct ConditionWrapping;

impl Rule for ConditionWrapping {
    fn id(&self) -> &'static str {
        "standard:condition-wrapping"
    }

    fn check(&self, _tree: &tree_sitter::Tree, _source: &str) -> Vec<Violation> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    #[test]
    fn condition_wrapping_is_a_noop_in_ktlint_1_8() {
        let source = "if (first &&\n    second) {\n    Unit\n}\n";
        let tree = KotlinParser::new().parse(source);
        assert!(ConditionWrapping.check(&tree, source).is_empty());
    }
}
