//! Naming rules — class, function, property, filename, enum conventions.
//!
//! All naming rules are configurable via .editorconfig:
//! - Ignore annotated: `ktlint_<rule>_ignore_when_annotated_with = Composable, Test`

pub mod class_naming;
pub mod filename;
pub mod function_naming;
pub mod property_naming;

pub use class_naming::ClassNaming;
pub use filename::Filename;
pub use function_naming::FunctionNaming;
pub use property_naming::PropertyNaming;
