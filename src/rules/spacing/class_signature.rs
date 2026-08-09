//! standard:class-signature — spacing around class signature components.
//!
//! Checks:
//! - Space before `:` in super type list
//! - Constructor parameter spacing
//! - Class body `{` positioning

use crate::config::CodeStyle;
use crate::rules::{Rule, Violation};

pub struct ClassSignatureSpacing {
    code_style: CodeStyle,
    max_line_length: usize,
}

impl ClassSignatureSpacing {
    pub fn new(code_style: CodeStyle, max_line_length: usize) -> Self {
        let max_line_length = if max_line_length == 0 {
            120
        } else {
            max_line_length
        };
        Self {
            code_style,
            max_line_length,
        }
    }
}

impl Rule for ClassSignatureSpacing {
    fn id(&self) -> &'static str {
        "standard:class-signature"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl ClassSignatureSpacing {
    fn walk(&self, node: tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        if node.kind() == "class_declaration" {
            self.check_class(&node, bytes, violations);
            // Issue #167: under android_studio, a multiline class parameter
            // list is reported (ktlint_official allows it).
            if self.code_style == CodeStyle::AndroidStudio {
                self.check_multiline_parameters(&node, bytes, violations);
            }
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, bytes, violations);
            }
        }
    }

    /// android_studio: a multiline class parameter list (`class Alpha(\n
    /// val name: String,\n)`) reports every parameter — the first with "No
    /// whitespace expected between opening parenthesis and first parameter
    /// name", the rest with "Single whitespace expected before parameter",
    /// and the last one's line end with "No whitespace expected between last
    /// parameter and closing parenthesis".
    fn check_multiline_parameters(
        &self,
        node: &tree_sitter::Node,
        bytes: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        // `class Alpha(...)` puts the parameter list inside a
        // primary_constructor whose direct children are `class_parameter`
        // nodes.
        let Some(ctor) = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "primary_constructor")
        else {
            return;
        };
        if ctor.start_position().row == ctor.end_position().row {
            return;
        }
        // Issue #177: android_studio asks for the single-line form only when
        // the collapsed signature actually fits within max_line_length — a
        // signature that cannot fit must stay multiline. Mirrors ktlint 1.8
        // (and function-signature's own fits check).
        if !self.collapsed_fits(node, &ctor, bytes) {
            return;
        }
        let mut w = ctor.walk();
        let params_nodes: Vec<tree_sitter::Node> = ctor
            .children(&mut w)
            .filter(|c| c.kind() == "class_parameter")
            .collect();
        for (idx, p) in params_nodes.iter().enumerate() {
            let pos = p.start_position();
            let message = if idx == 0 {
                "No whitespace expected between opening parenthesis and first parameter name"
            } else {
                "Single whitespace expected before parameter"
            };
            violations.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: self.id().to_string(),
                message: message.to_string(),
                auto_fixable: true,
            });
        }
        // Last parameter's line end (ktlint reports the column of the last
        // non-whitespace character, e.g. the trailing comma).
        if let Some(last) = params_nodes.last() {
            let row = last.end_position().row;
            let line_start = bytes[..last.end_byte()]
                .iter()
                .rposition(|&b| b == b'\n')
                .map_or(0, |i| i + 1);
            let line_end = bytes[line_start..]
                .iter()
                .position(|&b| b == b'\n')
                .map_or(bytes.len(), |i| line_start + i);
            let trimmed = bytes[line_start..line_end]
                .iter()
                .rposition(|&b| b != b' ' && b != b'\t')
                .map_or(line_start, |i| line_start + i);
            violations.push(Violation {
                file: String::new(),
                line: row + 1,
                col: trimmed - line_start + 2,
                rule_id: self.id().to_string(),
                message: "No whitespace expected between last parameter and closing parenthesis"
                    .to_string(),
                auto_fixable: true,
            });
        }
    }

    /// Issue #177: whether collapsing this class header onto one line would
    /// fit within max_line_length. The header runs from the declaration start
    /// (class keyword, or leading annotations) to just before the class body
    /// `{`; its width is the line indent plus the header text with every
    /// whitespace run collapsed to a single space (the form ktlint would
    /// produce).
    fn collapsed_fits(
        &self,
        node: &tree_sitter::Node,
        ctor: &tree_sitter::Node,
        bytes: &[u8],
    ) -> bool {
        let header_end = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "class_body")
            .map_or(node.end_byte(), |b| b.start_byte());
        let start = node.start_byte();
        if header_end <= start {
            return false;
        }
        // ctor end must fall within the header (it does for a primary
        // constructor).
        if ctor.end_byte() > header_end {
            return false;
        }
        let text = match std::str::from_utf8(&bytes[start..header_end]) {
            Ok(t) => t,
            Err(_) => return false,
        };
        // Indent of the declaration's first line.
        let line_start = bytes[..start]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        let indent_len = bytes[line_start..start]
            .iter()
            .filter(|&&b| b == b' ' || b == b'\t')
            .count();
        let collapsed_len = text
            .split_whitespace()
            .map(|w| w.chars().count())
            .sum::<usize>()
            + text.split_whitespace().count().saturating_sub(1);
        indent_len + collapsed_len <= self.max_line_length
    }

    fn check_class(&self, node: &tree_sitter::Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let mut saw_class_keyword = false;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let kind = child.kind();
                if kind == "class" {
                    saw_class_keyword = true;
                }
                // After class name and optional constructor, check `:` for super types

                // : in super type delegation
                if saw_class_keyword && kind == ":" {
                    // This `:` is in the delegation specifier (super type list)
                    // Should have space before and after
                    let pos = child.start_position();
                    let start_byte = child.start_byte();
                    let end_byte = child.end_byte();

                    // Space before
                    if start_byte > 0
                        && bytes[start_byte - 1] != b' '
                        && bytes[start_byte - 1] != b'\n'
                    {
                        violations.push(Violation {
                            file: String::new(),
                            line: pos.row + 1,
                            col: pos.column,
                            rule_id: self.id().to_string(),
                            message: "Missing space before \":\" in super type list".to_string(),
                            auto_fixable: true,
                        });
                    }
                    // Space after
                    if end_byte < bytes.len() && bytes[end_byte] != b' ' && bytes[end_byte] != b'\n'
                    {
                        violations.push(Violation {
                            file: String::new(),
                            line: pos.row + 1,
                            col: pos.column + 2,
                            rule_id: self.id().to_string(),
                            message: "Missing space after \":\" in super type list".to_string(),
                            auto_fixable: true,
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        ClassSignatureSpacing::new(CodeStyle::AndroidStudio, 120).check(&tree, source)
    }

    fn check_with_max(source: &str, max: usize) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        ClassSignatureSpacing::new(CodeStyle::AndroidStudio, max).check(&tree, source)
    }

    #[test]
    fn valid_class_signature() {
        assert!(check("class Foo : Bar\n").is_empty());
    }

    #[test]
    fn missing_space_before_super_colon() {
        let v = check("class Foo: Bar\n");
        assert!(!v.is_empty());
        assert!(v.iter().any(|x| x.message.contains("before")));
    }

    #[test]
    fn no_super_type_is_fine() {
        assert!(check("class Foo\n").is_empty());
    }

    #[test]
    fn class_with_constructor_and_super() {
        assert!(check("class Foo(val x: Int) : Bar(x)\n").is_empty());
    }

    // Issue #177: android_studio must only demand the single-line form when
    // the collapsed signature fits within max_line_length.
    #[test]
    fn multiline_params_report_when_collapsed_fits() {
        // `class Wide(private val alpha: String, private val beta: String)`
        // fits in 120 — ktlint asks for the collapse.
        let src = "class Wide(\n    private val alpha: String,\n    private val beta: String,\n)\n";
        assert!(!check(src).is_empty());
    }

    #[test]
    fn multiline_params_silent_when_collapsed_too_long() {
        // Collapsed form is 189 chars > 120 — must stay multiline, no report.
        let src = "class Wide(\n    private val alphaConfigurationValue: String,\n    private val betaConfigurationValue: String,\n    private val gammaConfigurationValue: String,\n    private val deltaConfigurationValue: String,\n)\n";
        let v = check(src);
        assert!(
            v.is_empty(),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn multiline_params_boundary_honours_max_length() {
        // A tight max_line_length must silence the report for a signature
        // that only fits a shorter collapsed form.
        let src = "class Wide(\n    private val alpha: String,\n    private val beta: String,\n)\n";
        // collapsed form is 66 chars; at 65 it does not fit → silent.
        assert!(check_with_max(src, 65).is_empty());
        assert!(!check_with_max(src, 66).is_empty());
    }

    #[test]
    fn multiline_params_nested_class_counts_indent() {
        // A nested class's collapsed signature includes its line indent.
        let src = "class Outer {\n    class Inner(\n        val a: String,\n    )\n}\n";
        assert!(!check(src).is_empty());
    }
}
