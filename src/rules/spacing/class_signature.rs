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

/// The byte just past the last non-newline char on `byte`'s line — the
/// closing-paren line may continue with ` {`, `: Int {` or ` : Super {`,
/// which must count against max_line_length (issue #188).

/// Length of ktlint's collapsed single-line form: each line trimmed and
/// joined with no separator, except after a trailing comma where ktlint
/// writes `, ` (its reformatted parameter list).
fn collapsed_len_of(text: &str) -> usize {
    let lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();
    let mut len = 0usize;
    for (i, l) in lines.iter().enumerate() {
        len += l.chars().count();
        if i + 1 < lines.len() && l.ends_with(',') && lines[i + 1] != ")" {
            len += 1;
        }
    }
    len
}

fn line_end_after(bytes: &[u8], byte: usize) -> usize {
    let line_end = bytes[byte..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |i| byte + i);
    // trim trailing whitespace on the line
    let mut end = line_end;
    while end > byte && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
        end -= 1;
    }
    end
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
                self.check_supertype_newline(&node, bytes, violations);
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
        let start = self.measure_start(node, bytes);
        // Collapse decision: signature + ` {` when the body opens on the
        // same line; the supertype list in between is not counted (issue
        // #188).
        if !self.collapse_fits(start, &ctor, node, bytes) {
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

    /// android_studio: when the full header (params + supertypes) cannot fit
    /// on one line, the supertype list must start on its own line — ktlint
    /// 1.8 reports "Super type should start on a newline" at the first
    /// supertype. Runs for every class (single-line params included); stays
    /// silent when the params themselves cannot collapse (ktlint does not
    /// demand a supertype newline then).
    fn check_supertype_newline(
        &self,
        node: &tree_sitter::Node,
        bytes: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        let supertypes: Vec<tree_sitter::Node> = node
            .children(&mut node.walk())
            .filter(|c| c.kind() == "delegation_specifier")
            .collect();
        let (Some(first), Some(last)) = (supertypes.first(), supertypes.last()) else {
            return;
        };
        let start = self.measure_start(node, bytes);
        // Full header (params + supertypes + body) on one line: fits → fine.
        if self.fits(start, line_end_after(bytes, last.end_byte()), node, bytes) {
            return;
        }
        // The supertype request only applies when the collapsed params
        // themselves fit (a multiline ctor that cannot collapse keeps the
        // whole header multiline — matches ktlint).
        if let Some(ctor) = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "primary_constructor")
        {
            if ctor.start_position().row != ctor.end_position().row
                && !self.collapse_fits(start, &ctor, node, bytes)
            {
                return;
            }
        }
        // The supertype name counts as already on a new line when a line
        // break sits between the `:` and the first supertype.
        let colon_end = node
            .children(&mut node.walk())
            .find(|c| c.kind() == ":")
            .map_or(start, |c| c.end_byte());
        if bytes[colon_end..first.start_byte()].contains(&b'\n') {
            return;
        }
        let pos = first.start_position();
        violations.push(Violation {
            file: String::new(),
            line: pos.row + 1,
            col: pos.column + 1,
            rule_id: self.id().to_string(),
            message: "Super type should start on a newline".to_string(),
            auto_fixable: true,
        });
    }

    /// The `class` keyword child of a class declaration.
    fn class_keyword<'a>(&self, node: &tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        node.children(&mut node.walk())
            .find(|c| c.kind() == "class")
    }

    /// Issue #177: whether collapsing `bytes[start..end]` (the class
    /// signature, with or without supertypes) onto one line fits within
    /// max_line_length. Mirrors ktlint 1.8's measurement: from the `class`
    /// keyword to the end of the primary constructor (annotations and
    /// modifiers are not part of it — verified against the oracle). Width is
    /// the line indent plus the header text with every whitespace run
    /// collapsed to a single space.
    /// Byte offset where ktlint starts measuring the collapsed signature:
    /// the first non-annotation token of the `modifiers` node (it may hold
    /// annotations before the actual modifiers — `@Deprecated("x")
    /// public class`), else the `class` keyword. Annotations are not part of
    /// the measurement (verified: a long annotation does not stop the
    /// collapse request), modifiers are (issue #182).

    /// The byte just past the last non-newline char on `byte`'s line — the
    /// closing-paren line may continue with ` {`, `: Int {` or ` : Super {`,
    /// which must count against max_line_length (issue #188).
    fn line_end_after(bytes: &[u8], byte: usize) -> usize {
        let line_end = bytes[byte..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(bytes.len(), |i| byte + i);
        // trim trailing whitespace on the line
        let mut end = line_end;
        while end > byte && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
            end -= 1;
        }
        end
    }

    fn measure_start<'a>(&self, node: &tree_sitter::Node<'a>, bytes: &[u8]) -> usize {
        if let Some(mods) = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "modifiers")
        {
            let text = &bytes[mods.start_byte()..mods.end_byte()];
            let mut i = 0usize;
            while i < text.len() && text[i] == b'@' {
                i += 1;
                while i < text.len()
                    && (text[i].is_ascii_alphanumeric() || text[i] == b'_' || text[i] == b'.')
                {
                    i += 1;
                }
                if i < text.len() && text[i] == b'(' {
                    let mut depth = 1usize;
                    i += 1;
                    while i < text.len() && depth > 0 {
                        match text[i] {
                            b'(' => depth += 1,
                            b')' => depth = depth.saturating_sub(1),
                            _ => {}
                        }
                        i += 1;
                    }
                }
                while i < text.len() && text[i].is_ascii_whitespace() {
                    i += 1;
                }
            }
            if i < text.len() {
                return mods.start_byte() + i;
            }
        }
        self.class_keyword(node)
            .map_or(node.start_byte(), |k| k.start_byte())
    }

    fn fits(&self, start: usize, end: usize, node: &tree_sitter::Node, bytes: &[u8]) -> bool {
        self.measurement_len(start, end, node, bytes) <= self.max_line_length
    }

    /// ktlint's collapsed single-line length of `bytes[start..end]`: line
    /// indent plus each line trimmed, joined with `, ` after a trailing
    /// comma (`class C(\n    val a: X,\n)` -> `class C(val a: X,)`).
    fn measurement_len(
        &self,
        start: usize,
        end: usize,
        node: &tree_sitter::Node,
        bytes: &[u8],
    ) -> usize {
        if end <= start {
            return usize::MAX;
        }
        let text = match std::str::from_utf8(&bytes[start..end]) {
            Ok(t) => t,
            Err(_) => return usize::MAX,
        };
        let decl_start = node.start_byte();
        let line_start = bytes[..decl_start]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        let indent_len = bytes[line_start..decl_start]
            .iter()
            .filter(|&&b| b == b' ' || b == b'\t')
            .count();
        indent_len + collapsed_len_of(text)
    }

    /// Collapse decision: the signature (to the closing paren) plus the
    /// class body's ` {` when it opens on the same line — the supertype list
    /// in between is not part of it (issue #188).
    fn collapse_fits(
        &self,
        start: usize,
        ctor: &tree_sitter::Node,
        node: &tree_sitter::Node,
        bytes: &[u8],
    ) -> bool {
        let base = self.measurement_len(start, ctor.end_byte(), node, bytes);
        if base > self.max_line_length {
            return false;
        }
        let body_on_line = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "class_body")
            .is_some_and(|b| b.start_position().row == ctor.end_position().row);
        if !body_on_line {
            return true;
        }
        // ` {` on the closing-paren line adds 2 columns.
        base + 2 <= self.max_line_length
    }

    /// Issue #177: whether collapsing this class header onto one line would
    /// fit within max_line_length. Mirrors ktlint 1.8's measurement: the
    /// signature runs from the `class` keyword to the end of the primary
    /// constructor — annotations, modifiers and the supertype list are not
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
        // collapsed form is 64 chars, trailing comma kept
        // (`class Wide(private val alpha: String, private val beta:
        // String,)` — oracle-verified: max=63 silent, max=64 reports).
        assert!(check_with_max(src, 63).is_empty());
        assert!(!check_with_max(src, 64).is_empty());
    }

    #[test]
    fn multiline_params_nested_class_counts_indent() {
        // A nested class's collapsed signature includes its line indent.
        let src = "class Outer {\n    class Inner(\n        val a: String,\n    )\n}\n";
        assert!(!check(src).is_empty());
    }

    // android_studio: when the full header (params + supertypes) cannot fit
    // on one line, the supertype list must start on its own line.
    #[test]
    fn supertype_newline_reported_when_full_header_too_long() {
        // Params collapse fits; with the supertype the line exceeds 120.
        let src = "class ChildWithLongName(\n    private val alphaConfigurationValue: String,\n    private val betaConfigurationValue: String,\n) : ParentBase()\n";
        let v = check(src);
        assert!(
            v.iter()
                .any(|x| x.message == "Super type should start on a newline"),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn supertype_newline_reported_for_single_line_params() {
        // Single-line params, supertype pushes the line over the limit.
        let src = "class Foo(val x: Int) : SomeExtremelyLongBaseClassNameThatExceedsTheLimitCompletelyAndThenSomeMoreCharactersToPushItOverTheEdgeCompletely()\n";
        let v = check(src);
        assert!(
            v.iter()
                .any(|x| x.message == "Super type should start on a newline"),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn supertype_newline_silent_when_supertype_already_on_new_line() {
        let src = "class Foo(\n    val x: Int,\n) :\n    SomeExtremelyLongBaseClassNameThatExceedsTheLimitCompletelyAndThenSomeMoreCharactersToPushItOverTheEdgeCompletely()\n";
        let v = check(src);
        assert!(
            v.iter()
                .all(|x| x.message != "Super type should start on a newline"),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn supertype_newline_silent_when_params_cannot_collapse() {
        // Params alone exceed the limit — ktlint stays silent about the
        // supertype too.
        let src = "class Huge(\n    private val alphaConfigurationValue: String,\n    private val betaConfigurationValue: String,\n    private val gammaConfigurationValue: String,\n    private val deltaConfigurationValue: String,\n) : SomeLongParent()\n";
        let v = check(src);
        assert!(
            v.iter()
                .all(|x| x.message != "Super type should start on a newline"),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn supertype_newline_silent_when_full_header_fits() {
        let src = "class Foo(val x: Int) : Parent()\n";
        assert!(check(src).is_empty());
    }

    #[test]
    fn supertype_newline_no_params() {
        // No constructor at all, supertype pushes the line over the limit.
        let src = "class Foo : SomeExtremelyLongBaseClassNameThatExceedsTheLimitCompletelyAndThenSomeMoreCharactersToPushItOverTheEdgeCompletely()\n";
        let v = check(src);
        assert!(
            v.iter()
                .any(|x| x.message == "Super type should start on a newline"),
            "violations: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn ktlint_official_never_requests_collapse() {
        // The multiline-param collapse is android_studio-only; under
        // ktlint_official the same shape must stay silent.
        let src =
            "class Short(\n    private val alpha: String,\n    private val beta: String,\n)\n";
        let mut parser = KotlinParser::new();
        let tree = parser.parse(src);
        let v = ClassSignatureSpacing::new(CodeStyle::KtlintOfficial, 120).check(&tree, src);
        assert!(
            v.is_empty(),
            "ktlint_official must not request collapse: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn supertype_newline_gated_on_android_studio() {
        // "Super type should start on a newline" is android_studio-only.
        let src = "class Foo(val x: Int) : SomeExtremelyLongBaseClassNameThatExceedsTheLimitCompletelyAndThenSomeMoreCharactersToPushItOverTheEdgeCompletely()\n";
        let mut parser = KotlinParser::new();
        let tree = parser.parse(src);
        let v = ClassSignatureSpacing::new(CodeStyle::KtlintOfficial, 120).check(&tree, src);
        assert!(
            v.iter()
                .all(|x| x.message != "Super type should start on a newline"),
            "ktlint_official must not report supertype newline: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn multiline_params_silent_when_max_unset_defaults_to_120() {
        // max_line_length = 0 means "default 120" (same as MaxLineLength).
        let src = "class Wide(\n    private val alphaConfigurationValue: String,\n    private val betaConfigurationValue: String,\n    private val gammaConfigurationValue: String,\n    private val deltaConfigurationValue: String,\n)\n";
        let mut parser = KotlinParser::new();
        let tree = parser.parse(src);
        let v = ClassSignatureSpacing::new(CodeStyle::AndroidStudio, 0).check(&tree, src);
        assert!(
            v.is_empty(),
            "collapsed width 189 > default 120 must stay silent: {:?}",
            v.iter().map(|x| (&x.message, x.line)).collect::<Vec<_>>()
        );
    }
}
