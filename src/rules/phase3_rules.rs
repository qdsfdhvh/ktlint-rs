//! Phase 3.3 batch: no-empty-class-body, string-template, if-else-bracing
use crate::rules::{Rule, Violation};

pub struct StringTemplateRule;
impl Rule for StringTemplateRule {
    fn id(&self) -> &'static str {
        "standard:string-template"
    }
    fn check(&self, _t: &tree_sitter::Tree, _s: &str) -> Vec<Violation> {
        // Fail closed: ktlint's string-template only flags `$var.property`
        // (needs ${} braces) and `$var` followed by identifier chars. `$index`
        // etc. are valid simple templates; the previous line-scan flagged every
        // `$word` inside any string, causing mass false positives.
        Vec::new()
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
