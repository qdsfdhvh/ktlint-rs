//! detekt static analysis rules — native Kotlin checks that go beyond ktlint formatting.
//!
//! Organized by detekt categories:
//! - `empty-blocks` (14) — flag empty code blocks
//! - `complexity` (7) — LongMethod, Cyclomatic, etc.
//! - `naming` (4) — Max/MinLength, Enum, Parameter
//! - `comments` (3) — Deprecated, Sentence, License
//! - `style` (6) — NoTabs, Forbidden, Wildcard, MandatoryBraces, PackageSpacing
//! - `potential-bugs` (3) — DuplicateCaseInWhen, UnreachableCatch, EqualsNull

pub mod comments;
pub mod comments_l1;
pub mod complexity;
pub mod empty_blocks;
pub mod exceptions;
pub mod naming;
pub mod potential_bugs;
pub mod style;
