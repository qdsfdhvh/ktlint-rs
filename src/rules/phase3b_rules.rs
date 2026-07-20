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
        let lines: Vec<&str> = s.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let t = lines[i].trim();
            // Find block-body functions: fun name(...): Type {
            if t.starts_with("fun ") && t.ends_with('{') && !t.contains('=') {
                let _fun_line = i;
                i += 1;
                let mut depth = 1usize;
                let mut return_count = 0usize;
                let mut return_line = 0usize;
                let mut has_other_statements = false;
                while i < lines.len() && depth > 0 {
                    let body = lines[i].trim();
                    let opens = body.matches('{').count();
                    let closes = body.matches('}').count();
                    depth = depth + opens - closes;
                    if body.starts_with("return ") && !body.contains("//") {
                        return_count += 1;
                        return_line = i;
                    } else if !body.is_empty() && !body.starts_with("//") && body != "}" {
                        // Check if it's a real statement (not just a closing brace line)
                        if closes == 0 || body.trim_end_matches('}').trim().len() > 0 {
                            has_other_statements = true;
                        }
                    }
                    i += 1;
                }
                // Flag if exactly one return and no other statements in body
                if return_count == 1 && !has_other_statements {
                    v.push(Violation {
                        file: String::new(),
                        line: return_line + 1,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Function body should be replaced with body expression".into(),
                        auto_fixable: true,
                    });
                }
            } else {
                i += 1;
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
