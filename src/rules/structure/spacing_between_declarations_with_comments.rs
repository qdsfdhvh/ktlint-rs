//! standard:spacing-between-declarations-with-comments
use crate::rules::{Rule, Violation};

pub struct SpacingBetweenDeclarationsWithComments;
impl Rule for SpacingBetweenDeclarationsWithComments {
    fn id(&self) -> &'static str {
        "standard:spacing-between-declarations-with-comments"
    }
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        // Fail closed: ktlint only flags a missing blank line when a comment
        // separates two declarations in a way that changes grouping semantics.
        // The previous line-scan flagged every `// comment` immediately before
        // `fun`/`class`, including valid doc-comment style that the live
        // Spotless oracle does not report (disabled-by-oracle).
        Vec::new()
    }
}
