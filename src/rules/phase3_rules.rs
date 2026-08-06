//! Phase 3.3 batch: no-empty-class-body, string-template, if-else-bracing
use crate::rules::{Rule, Violation};

pub struct NoEmptyClassBody;
impl Rule for NoEmptyClassBody {
    fn id(&self) -> &'static str {
        "standard:no-empty-class-body"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        s.lines()
            .enumerate()
            .filter(|(_, l)| {
                let t = l.trim();
                (t.ends_with("{}") || t.ends_with("{ }"))
                    && (t.contains("class ") || t.contains("interface ") || t.contains("object "))
            })
            .map(|(i, _)| Violation {
                file: String::new(),
                line: i + 1,
                col: 1,
                rule_id: self.id().into(),
                message: "Empty class body is unnecessary".into(),
                auto_fixable: false,
            })
            .collect()
    }
}

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

pub struct BlankLineBeforeFileAnnotation;
impl Rule for BlankLineBeforeFileAnnotation {
    fn id(&self) -> &'static str {
        "standard:blank-line-before-file-annotation"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            if ln.trim().starts_with("@file:") && i > 0 && !l[i - 1].trim().is_empty() {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Blank line required before @file annotation".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct BlankLineBeforePackage;
impl Rule for BlankLineBeforePackage {
    fn id(&self) -> &'static str {
        "standard:blank-line-before-package"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            if (ln.trim() == "package" || ln.trim().starts_with("package "))
                && i > 0
                && !l[i - 1].trim().is_empty()
                && !l[i - 1].trim().starts_with("//")
            {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "File annotations must be placed before package".into(),
                    auto_fixable: true,
                });
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
