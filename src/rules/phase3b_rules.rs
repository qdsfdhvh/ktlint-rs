//! Phase 3.3 semantics: no-empty-file, indent, max-line, kdoc, function-signature
use crate::rules::{Rule, Violation};

pub struct NoEmptyFile;
impl Rule for NoEmptyFile {
    fn id(&self) -> &'static str {
        "standard:no-empty-file"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        if s.trim().is_empty() {
            vec![Violation {
                file: String::new(),
                line: 1,
                col: 1,
                rule_id: self.id().into(),
                message: "File must not be empty".into(),
                auto_fixable: false,
            }]
        } else {
            vec![]
        }
    }
}

pub struct FunctionSignatureSpacing;
impl Rule for FunctionSignatureSpacing {
    fn id(&self) -> &'static str {
        "standard:function-signature"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            let t = ln.trim();
            if t.starts_with("fun ") && t.contains('(') && !t.contains(')') {
                if !t.ends_with(',') {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Newline expected after opening parenthesis".into(),
                        auto_fixable: true,
                    });
                }
            }
        }
        v
    }
}

pub struct FunctionExpressionBody;
impl Rule for FunctionExpressionBody {
    fn id(&self) -> &'static str {
        "standard:function-expression-body"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            let t = l.trim();
            if t.starts_with("fun ")
                && t.contains('=')
                && !t.contains('{')
                && !t.contains("return ")
            {
                let body = t.split('=').nth(1).unwrap_or("").trim();
                if !body.is_empty() {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Function body should be replaced with body expression".into(),
                        auto_fixable: true,
                    });
                }
            }
        }
        v
    }
}

pub struct KeywordSpacing;
impl Rule for KeywordSpacing {
    fn id(&self) -> &'static str {
        "standard:keyword-spacing"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.contains("if(")
                || l.contains("for(")
                || l.contains("while(")
                || l.contains("when(")
            {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Missing space after keyword".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}

pub struct ParameterListSpacingRule;
impl Rule for ParameterListSpacingRule {
    fn id(&self) -> &'static str {
        "standard:parameter-list-spacing"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.contains(" ,") || l.contains(",  ") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Whitespace around comma is inconsistent".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}
