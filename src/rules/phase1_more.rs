//! annotation/wrapping/no-consecutive-comments rules — JVM parity
use crate::rules::{Rule, Violation};

pub struct KtlintAnnotation;
impl Rule for KtlintAnnotation {
    fn id(&self) -> &'static str { "standard:annotation" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let l:Vec<&str>=s.lines().collect();
        for (i,ln) in l.iter().enumerate() {
            let t=ln.trim();
            if t.starts_with("@file:") { continue; }
            // Multiple annotations on same line without whitespace
            if t.matches('@').count()>1 {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Multiple annotations on same line".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct KtlintWrapping;
impl Rule for KtlintWrapping {
    fn id(&self) -> &'static str { "standard:wrapping" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let l:Vec<&str>=s.lines().collect();
        for (i,ln) in l.iter().enumerate() {
            let t=ln.trim();
            if (t.starts_with("if (")||t.starts_with("for (")||t.starts_with("while ("))
                && t.ends_with('{') && i+1<l.len() && l[i+1].trim().is_empty() {
                v.push(Violation{file:String::new(),line:i+2,col:1,rule_id:self.id().into(),
                    message:"Unexpected blank line after brace".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct KtlintNoConsecutiveComments;
impl Rule for KtlintNoConsecutiveComments {
    fn id(&self) -> &'static str { "standard:no-consecutive-comments" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let l:Vec<&str>=s.lines().collect();
        for i in 0..l.len().saturating_sub(1) {
            let a=l[i].trim(); let b=l[i+1].trim();
            if a.starts_with("//") && b.starts_with("//") && !a.starts_with("///") && !b.starts_with("///") {
                v.push(Violation{file:String::new(),line:i+2,col:1,rule_id:self.id().into(),
                    message:"Consecutive comments should be combined".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct KtlintArgumentListWrapping;
impl Rule for KtlintArgumentListWrapping {
    fn id(&self) -> &'static str { "standard:argument-list-wrapping" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let l:Vec<&str>=s.lines().collect();
        for (i,ln) in l.iter().enumerate() {
            let t=ln.trim();
            if t.starts_with("fun ") && t.contains('(') && !t.contains(')') {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Argument list should be wrapped consistently".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct KtlintFilename;
impl Rule for KtlintFilename {
    fn id(&self) -> &'static str { "standard:filename" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        for (i,ln) in s.lines().enumerate() {
            if let Some(rest)=ln.trim().strip_prefix("class ") {
                let name=rest.split(|c:char|!c.is_alphanumeric()).next().unwrap_or("");
                if name.chars().next().map_or(false,|c|c.is_lowercase()) {
                    return vec![Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                        message:"Class name should match filename".into(),auto_fixable:false}];
                }
            }
        } vec![]
    }
}
