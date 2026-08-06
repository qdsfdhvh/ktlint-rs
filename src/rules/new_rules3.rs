//! Batch 3: wrapping, comment, expression rules
use crate::rules::{Rule, Violation};

pub struct EnumWrappingRule;
impl Rule for EnumWrappingRule {
    fn id(&self) -> &'static str {
        "standard:enum-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        let mut in_enum = false;
        for (i, ln) in l.iter().enumerate() {
            let t = ln.trim();
            if t.starts_with("enum ") {
                in_enum = true;
            }
            if in_enum && t == "}" {
                in_enum = false;
            }
            if in_enum && t.starts_with('{') && t.contains(',') {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Enum entries should be on separate lines".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct NoEmptyFirstLineInMethodBlockRule;
impl Rule for NoEmptyFirstLineInMethodBlockRule {
    fn id(&self) -> &'static str {
        "standard:no-empty-first-line-in-method-block"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            if (ln.trim().starts_with("fun ") || ln.trim().starts_with("init {"))
                && ln.trim().ends_with('{')
                && i + 1 < l.len()
                && l[i + 1].trim().is_empty()
            {
                v.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Unexpected blank line at start of method body".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct TrailingCommaOnDeclarationSiteRule;
impl Rule for TrailingCommaOnDeclarationSiteRule {
    fn id(&self) -> &'static str {
        "standard:trailing-comma-on-declaration-site"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            let t = l.trim();
            if (t.starts_with("data class ") || t.starts_with("class "))
                && t.contains(',')
                && t.contains(')')
            {
                if let Some(rp) = t.rfind(')') {
                    if rp > 1 && t.as_bytes()[rp - 1] == b',' {
                        v.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: rp + 1,
                            rule_id: self.id().into(),
                            message: "Trailing comma on declaration site".into(),
                            auto_fixable: true,
                        });
                    }
                }
            }
        }
        v
    }
}

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
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        // Fail closed: the previous line-scan heuristic produced mass false
        // positives on real projects (verified against a live Spotless 8.8.0 +
        // ktlint 1.8.0 oracle with zero violations). A CST-aware implementation
        // must replace this before the rule can be re-enabled.
        Vec::new()
    }
}

pub struct LambdaReturnRule;
impl Rule for LambdaReturnRule {
    fn id(&self) -> &'static str {
        "standard:lambda-return"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.contains("return@") && l.contains("return@") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Use implicit return in lambdas".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct BlankLineBetweenWhenConditionsRule;
impl Rule for BlankLineBetweenWhenConditionsRule {
    fn id(&self) -> &'static str {
        "standard:blank-line-between-when-conditions"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        let mut in_when = false;
        for i in 0..l.len() {
            let t = l[i].trim();
            if t.starts_with("when ") {
                in_when = true;
            }
            if in_when && t == "}" {
                in_when = false;
            }
            if in_when
                && t.contains("->")
                && i + 1 < l.len()
                && !l[i + 1].trim().is_empty()
                && l[i + 1].trim().contains("->")
            {
                v.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Consider blank line between when conditions".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}
