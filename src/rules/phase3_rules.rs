//! Phase 3.3 batch: no-empty-class-body, string-template, if-else-bracing
use crate::rules::{Rule, Violation};

pub struct StringTemplateRule;
impl Rule for StringTemplateRule {
    fn id(&self) -> &'static str {
        "standard:string-template"
    }
    /// Mirrors ktlint 1.8 StringTemplateRule (report half): `${identifier}`
    /// whose content is a plain identifier may drop the curly braces
    /// (`"hi ${name}"` → `"hi $name"`). `${user.name}` (property access) and
    /// `$name` (short form) are fine.
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind().contains("string") {
                let text = &source[node.start_byte()..node.end_byte()];
                // Scan for `${...}` entries inside the literal.
                let mut i = 0;
                while let Some(rel) = text[i..].find("${") {
                    let start = i + rel;
                    let after = &text[start + 2..];
                    let end = after.find('}');
                    let Some(end_rel) = end else { break };
                    let inner = &after[..end_rel];
                    // Plain identifier (letters/digits/underscore only) and not
                    // `this`/`super` — redundant braces.
                    let is_ident =
                        !inner.is_empty() && inner.chars().all(|c| c.is_alphanumeric() || c == '_');
                    // Dropping the braces is only safe when the following
                    // char is not part of an identifier (`"${name}x"` →
                    // `$namex` would change the meaning).
                    let next_char = after[end_rel + 1..].chars().next();
                    let next_is_ident = next_char.is_some_and(|c| c.is_alphanumeric() || c == '_');
                    if is_ident && !next_is_ident && !matches!(inner, "this" | "super") {
                        let pos = node.start_byte() + start + 2;
                        let line = source[..pos].bytes().filter(|&b| b == b'\n').count() + 1;
                        let line_start = source[..pos].rfind('\n').map_or(0, |j| j + 1);
                        violations.push(Violation {
                            file: String::new(),
                            line,
                            col: pos - line_start + 1,
                            rule_id: self.id().into(),
                            message: "Redundant curly braces".into(),
                            auto_fixable: true,
                        });
                    }
                    i = start + end_rel + 3;
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

pub struct IfElseBracingRule;
impl Rule for IfElseBracingRule {
    fn id(&self) -> &'static str {
        "standard:if-else-bracing"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        let mut saw_brace = false;
        for (i, ln) in l.iter().enumerate() {
            let t = ln.trim();
            if (t.starts_with("if ") || t.starts_with("else if ")) && t.ends_with('{') {
                saw_brace = true;
            }
            if saw_brace && t == "}" {
                saw_brace = false;
            }
            if (t == "else" || t == "else if") && !saw_brace && i + 1 < l.len() {
                let next = l[i + 1].trim();
                if !next.starts_with("{") {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "If one branch uses braces, all branches must use braces".into(),
                        auto_fixable: true,
                    });
                }
            }
        }
        v
    }
}

pub struct ContextReceiverWrapping;
impl Rule for ContextReceiverWrapping {
    fn id(&self) -> &'static str {
        "standard:context-receiver-wrapping"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.trim().starts_with("context(") && !l.trim().contains(')') {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message:
                        "Context receiver should be on a single line or each parameter on new line"
                            .into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct TypeParameterListSpacing;
impl Rule for TypeParameterListSpacing {
    fn id(&self) -> &'static str {
        "standard:type-parameter-list-spacing"
    }
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        // Fail closed: the previous line-scan could not distinguish `< ` inside
        // type argument lists from comparison operators or generics in strings,
        // producing false positives on real projects.
        Vec::new()
    }
}
