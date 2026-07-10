//! Wrapping rules — chain wrapping, when/if/for/while brace placement.

pub mod chain_wrapping;
pub mod multiline_expression;
pub mod multiline_if_else;
pub mod string_template_indent;

pub use chain_wrapping::ChainWrapping;
pub use multiline_expression::MultilineExpressionWrapping;
pub use multiline_if_else::MultilineIfElse;
pub use string_template_indent::StringTemplateIndent;
