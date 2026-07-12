//! Phase 3 final: type-argument-list, angle-brackets, function-signature, enum-wrapping, trailing-comma-*
use crate::rules::{Rule, Violation};

pub struct TypeArgumentListSpacing;
impl Rule for TypeArgumentListSpacing {
    fn id(&self) -> &'static str { "standard:type-argument-list-spacing" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new();
        for (i,l) in s.lines().enumerate() {
            if l.contains("< ") && l.contains(">") && !l.contains("\"") && !l.contains("->") {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"No whitespace expected at this position".into(),auto_fixable:true});
            }
            if l.contains(" >") && l.contains("<") && !l.contains("->") && !l.contains("\"") {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"No whitespace expected at this position".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct SpacingAroundAngleBrackets;
impl Rule for SpacingAroundAngleBrackets {
    fn id(&self) -> &'static str { "standard:spacing-around-angle-brackets" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new();
        for (i,l) in s.lines().enumerate() {
            let t=l.trim();
            if t.contains("< ") && !t.contains("\"") && !t.contains("<-") {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Unexpected spacing after \"<\"".into(),auto_fixable:true});
            }
            if t.contains(" >") && !t.contains("->") && !t.contains("\"") {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Unexpected spacing before \">\"".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct EnumWrapping;
impl Rule for EnumWrapping {
    fn id(&self) -> &'static str { "standard:enum-wrapping" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let l:Vec<&str>=s.lines().collect();
        let mut in_enum=false;
        for (i,ln) in l.iter().enumerate() {
            let t=ln.trim();
            if t.starts_with("enum ") { in_enum=true; }
            if in_enum && t=="}" { in_enum=false; }
            if in_enum && t.starts_with('{') && t.contains(',') {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Enum entry should start on a separate line".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct TrailingCommaOnDeclarationSite;
impl Rule for TrailingCommaOnDeclarationSite {
    fn id(&self) -> &'static str { "standard:trailing-comma-on-declaration-site" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new();
        for (i,l) in s.lines().enumerate() {
            let t=l.trim();
            if (t.starts_with("data class ")||t.starts_with("class ")) && t.contains(')') {
                if let Some(rp)=t.rfind(')') {
                    if rp>1 && t.as_bytes()[rp-1]==b',' {
                        v.push(Violation{file:String::new(),line:i+1,col:rp+1,rule_id:self.id().into(),
                            message:"Missing trailing comma on declaration site".into(),auto_fixable:true});
                    }
                }
            }
        } v
    }
}

pub struct TrailingCommaOnCallSite;
impl Rule for TrailingCommaOnCallSite {
    fn id(&self) -> &'static str { "standard:trailing-comma-on-call-site" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new();
        for (i,l) in s.lines().enumerate() {
            if l.contains("(") && l.contains(")") && !l.trim().starts_with("fun ") && !l.trim().starts_with("class ") {
                if let Some(rp)=l.rfind(')') {
                    if rp>1 && l.as_bytes()[rp-1]==b',' {
                        v.push(Violation{file:String::new(),line:i+1,col:rp+1,rule_id:self.id().into(),
                            message:"Missing trailing comma on call site".into(),auto_fixable:true});
                    }
                }
            }
        } v
    }
}
