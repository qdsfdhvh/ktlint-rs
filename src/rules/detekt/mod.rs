//! detekt static analysis rules — native Kotlin checks that go beyond ktlint formatting.
//!
//! Organized by detekt categories:
//! - `empty-blocks` (14 rules) — flag empty code blocks
//! - `complexity` (4 rules) — measure code complexity (LongMethod, LongParameterList, NestedBlockDepth, LargeClass)

pub mod empty_blocks;
pub mod complexity;
