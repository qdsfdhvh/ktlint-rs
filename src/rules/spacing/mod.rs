//! Spacing rules — whitespace around tokens, braces, operators, and keywords.
//!
//! These rules account for ~80% of real-world Kotlin style violations.
//! Each rule traverses the tree-sitter CST to find relevant nodes and checks
//! whitespace around them.

pub mod annotation;
pub mod colon;
pub mod comma;
pub mod comment;
pub mod curly;
pub mod function_return_type;
pub mod function_start_body;
pub mod operator;
pub mod paren;

// Re-export for convenience
pub use annotation::AnnotationSpacing;
pub use colon::ColonSpacing;
pub use comma::CommaSpacing;
pub use comment::CommentSpacing;
pub use curly::CurlySpacing;
pub use function_return_type::FunctionReturnTypeSpacing;
pub use function_start_body::FunctionStartOfBodySpacing;
pub use operator::OperatorSpacing;
pub use paren::ParenSpacing;
