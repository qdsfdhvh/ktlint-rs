//! Batch 3: wrapping, comment, expression rules
use crate::rules::{Rule, Violation};

pub struct TypeArgumentCommentRule;
impl Rule for TypeArgumentCommentRule {
    fn id(&self) -> &'static str {
        "standard:type-argument-comment"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        // ktlint: a comment whose parent is a type_projection is disallowed;
        // a comment directly inside a type_argument_list is only allowed on
        // its own line.
        check_comment_rule(
            tree,
            source,
            &["type_projection"],
            &["type_arguments"],
            "A comment in a 'type_argument_list' is only allowed when placed on a separate line",
            "A (block or EOL) comment inside or on same line after a 'type_projection' is not allowed. It may be placed on a separate line above.",
            "standard:type-argument-comment",
        )
    }
}

pub struct TypeParameterCommentRule;
impl Rule for TypeParameterCommentRule {
    fn id(&self) -> &'static str {
        "standard:type-parameter-comment"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        check_comment_rule(
            tree,
            source,
            &["type_parameter"],
            &["type_parameters"],
            "A comment in a 'type_parameter_list' is only allowed when placed on a separate line",
            "A (block or EOL) comment inside or on same line after a 'type_parameter' is not allowed. It may be placed on a separate line above.",
            "standard:type-parameter-comment",
        )
    }
}

pub struct ValueArgumentCommentRule;
impl Rule for ValueArgumentCommentRule {
    fn id(&self) -> &'static str {
        "standard:value-argument-comment"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        // A trailing line comment after a value argument (`Color(0xFF), // pink`)
        // has the list as parent and is allowed; only a comment *inside* a
        // value_argument is reported.
        check_comment_rule(
            tree,
            source,
            &["value_argument"],
            &[],
            "A (block or EOL) comment inside or on same line after a 'value_argument' is not allowed. It may be placed on a separate line above.",
            "A (block or EOL) comment inside or on same line after a 'value_argument' is not allowed. It may be placed on a separate line above.",
            "standard:value-argument-comment",
        )
    }
}

pub struct ValueParameterCommentRule;
impl Rule for ValueParameterCommentRule {
    fn id(&self) -> &'static str {
        "standard:value-parameter-comment"
    }
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        check_comment_rule(
            tree,
            source,
            &["value_parameter", "class_parameter"],
            &[],
            "A comment inside or on same line after a 'value_parameter' is not allowed. It may be placed on a separate line above.",
            "A comment inside or on same line after a 'value_parameter' is not allowed. It may be placed on a separate line above.",
            "standard:value-parameter-comment",
        )
    }
}

/// Shared implementation mirroring ktlint's *-argument/parameter-comment
/// rules:
/// - a comment whose parent is one of `parent_kinds` (a single argument /
///   parameter / type projection) is always reported;
/// - a comment that is a direct child of one of `list_kinds` is reported only
///   when it is not on a line by itself.
fn check_comment_rule(
    tree: &tree_sitter::Tree,
    source: &str,
    parent_kinds: &[&str],
    list_kinds: &[&str],
    message: &str,
    parent_message: &str,
    rule_id: &'static str,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.kind().contains("comment") {
            let on_own_line = {
                let line_start = source[..node.start_byte()].rfind('\n').map_or(0, |i| i + 1);
                source[line_start..node.start_byte()].trim().is_empty()
            };
            let parent_kind = node.parent().map(|p| p.kind());
            let msg = if parent_kind.is_some_and(|k| parent_kinds.contains(&k)) {
                Some(parent_message)
            } else if on_own_line {
                None
            } else if parent_kind.is_some_and(|k| list_kinds.contains(&k)) {
                Some(message)
            } else {
                None
            };
            if let Some(msg) = msg {
                let line = source[..node.start_byte()]
                    .bytes()
                    .filter(|&b| b == b'\n')
                    .count()
                    + 1;
                let line_start = source[..node.start_byte()].rfind('\n').map_or(0, |i| i + 1);
                violations.push(Violation {
                    file: String::new(),
                    line,
                    col: node.start_byte() - line_start + 1,
                    rule_id: rule_id.into(),
                    message: msg.into(),
                    auto_fixable: false,
                });
            }
        }
        for i in (0..node.child_count()).rev() {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
    violations
}

pub struct ThenSpacingRule;
impl Rule for ThenSpacingRule {
    fn id(&self) -> &'static str {
        "standard:then-spacing"
    }
    /// Mirrors ktlint 1.8 ThenSpacingRule: the then-block of an if expression
    /// must be separated by whitespace — `if (x){` is reported.
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "if_expression" {
                // The then branch is the first child that starts with `{`
                // (or a statement). ktlint's THEN: whitespace before it.
                let mut w = node.walk();
                let kids: Vec<tree_sitter::Node> = node.children(&mut w).collect();
                for kid in kids {
                    let t = &source[kid.start_byte()..kid.end_byte()];
                    if t.trim_start().starts_with('{') {
                        // `)` (or condition end) and `{` must have whitespace.
                        let prev_end = kid
                            .prev_sibling()
                            .map(|p| p.end_byte())
                            .unwrap_or(kid.start_byte());
                        let between = &source[prev_end..kid.start_byte()];
                        if !between.chars().any(|c| c == ' ' || c == '\t' || c == '\n') {
                            let line = source[..kid.start_byte()]
                                .bytes()
                                .filter(|&b| b == b'\n')
                                .count()
                                + 1;
                            let line_start =
                                source[..kid.start_byte()].rfind('\n').map_or(0, |i| i + 1);
                            violations.push(Violation {
                                file: String::new(),
                                line,
                                col: kid.start_byte() - line_start + 1,
                                rule_id: self.id().into(),
                                message: "Expected a whitespace before 'then' block".into(),
                                auto_fixable: true,
                            });
                        }
                        break;
                    }
                }
            }
            for i in (0..node.child_count()).rev() {
                if let Some(child) = node.child(i) {
                    stack.push(child);
                }
            }
        }
        violations
    }
}
