//! detekt static analysis rules — native Kotlin checks that go beyond ktlint formatting.
//!
//! Organized by detekt categories:
//! - `empty-blocks` (14) — flag empty code blocks
//! - `complexity` (7) — LongMethod, NestedBlockDepth, etc.
//! - `naming` (3) — FunctionMax/MinLength, EnumNaming
//! - `comments` (3) — DeprecatedBlockTag, EndOfSentenceFormat, License
//! - `style` (4) — NoTabs, ForbiddenComment, WildcardImport, MandatoryBraces

pub mod empty_blocks;
pub mod complexity;
pub mod naming;
pub mod comments;
pub mod style;
