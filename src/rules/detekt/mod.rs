//! detekt static analysis rules — native Kotlin checks that go beyond ktlint formatting.
//!
//! Organized by detekt categories:
//! - `empty-blocks` (14 rules) — flag empty code blocks
//! - `complexity` (7 rules) — code complexity metrics
//! - `naming` (3 rules) — FunctionMax/MinLength, EnumNaming
//! - `comments` (3 rules) — DeprecatedBlockTag, EndOfSentenceFormat, License

pub mod empty_blocks;
pub mod complexity;
pub mod naming;
pub mod comments;
