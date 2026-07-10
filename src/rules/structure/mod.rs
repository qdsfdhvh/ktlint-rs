//! Structure rules — indentation, trailing whitespace, blank lines, etc.

pub mod enum_entry;
pub mod indentation;
pub mod max_line_length;
pub mod no_blank_line_before_rbrace;
pub mod no_empty_file;
pub mod parameter_list_spacing;
pub mod spacing_between_declarations;
pub mod trailing_comma;

pub use enum_entry::EnumEntry;
pub use indentation::Indentation;
pub use max_line_length::MaxLineLength;
pub use no_blank_line_before_rbrace::NoBlankLineBeforeRbrace;
pub use no_empty_file::NoEmptyFile;
pub use parameter_list_spacing::ParameterListSpacing;
pub use spacing_between_declarations::SpacingBetweenDeclarations;
pub use trailing_comma::TrailingComma;
