//! Spacing rules — whitespace around tokens, braces, operators, and keywords.

pub mod annotation;
pub mod class_signature;
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
pub use class_signature::ClassSignatureSpacing;
pub use colon::ColonSpacing;
pub use comma::CommaSpacing;
pub use comment::CommentSpacing;
pub use curly::CurlySpacing;
pub use function_return_type::FunctionReturnTypeSpacing;
pub use function_start_body::FunctionStartOfBodySpacing;
pub use operator::OperatorSpacing;
pub use paren::ParenSpacing;
