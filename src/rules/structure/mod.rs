//! Structure rules — indentation, trailing whitespace, blank lines, etc.

pub mod trailing_comma;
pub mod no_empty_file;
pub mod max_line_length;
pub mod no_blank_line_before_rbrace;
pub mod indentation;

pub use trailing_comma::TrailingComma;
pub use no_empty_file::NoEmptyFile;
pub use max_line_length::MaxLineLength;
pub use no_blank_line_before_rbrace::NoBlankLineBeforeRbrace;
pub use indentation::Indentation;
