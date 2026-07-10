//! Spacing rules — whitespace around tokens, braces, operators, and keywords.
//!
//! These rules account for ~80% of real-world Kotlin style violations.
//! Each rule traverses the tree-sitter CST to find relevant nodes and checks
//! whitespace around them.

pub mod curly;
pub mod operator;
pub mod comma;
pub mod paren;
pub mod colon;
pub mod annotation;
pub mod comment;
pub mod function_return_type;
pub mod function_start_body;

// Re-export for convenience
pub use curly::CurlySpacing;
pub use operator::OperatorSpacing;
pub use comma::CommaSpacing;
pub use paren::ParenSpacing;
pub use colon::ColonSpacing;
pub use annotation::AnnotationSpacing;
pub use comment::CommentSpacing;
pub use function_return_type::FunctionReturnTypeSpacing;
pub use function_start_body::FunctionStartOfBodySpacing;
