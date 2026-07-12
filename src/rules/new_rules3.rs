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
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.contains("/*") && l.contains("<") && l.contains(">") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Type argument comment should use KDoc format".into(),
                    auto_fixable: false,
                });
            }
        }
        v
    }
}

pub struct TypeParameterCommentRule;
impl Rule for TypeParameterCommentRule {
    fn id(&self) -> &'static str {
        "standard:type-parameter-comment"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.contains("<") && l.contains("//") && !l.contains("\"") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Type parameter comment should use /* */ or KDoc".into(),
                    auto_fixable: false,
                });
            }
        }
        v
    }
}

pub struct ValueArgumentCommentRule;
impl Rule for ValueArgumentCommentRule {
    fn id(&self) -> &'static str {
        "standard:value-argument-comment"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.contains("(") && l.contains("/*") && l.contains(")") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Value argument comment should use named parameters or KDoc".into(),
                    auto_fixable: false,
                });
            }
        }
        v
    }
}

pub struct ValueParameterCommentRule;
impl Rule for ValueParameterCommentRule {
    fn id(&self) -> &'static str {
        "standard:value-parameter-comment"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.contains("val ") && l.contains("/*") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Value parameter comment should use KDoc".into(),
                    auto_fixable: false,
                });
            }
        }
        v
    }
}

pub struct ThenSpacingRule;
impl Rule for ThenSpacingRule {
    fn id(&self) -> &'static str {
        "standard:then-spacing"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, l) in s.lines().enumerate() {
            if l.contains("?.let")
                || l.contains("?.run")
                || l.contains("?.apply")
                || l.contains("?.also")
            {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Use \".\" instead of \"?.\" when receiver is not null".into(),
                    auto_fixable: true,
                });
            }
        }
        v
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
