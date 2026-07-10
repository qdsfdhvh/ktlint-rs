//! Batch 2: ktlint parity rules (wrapping, declaration, spacing, comment)
use crate::rules::{Rule, Violation};

pub struct AnnotationRule;
impl Rule for AnnotationRule {
    fn id(&self) -> &'static str { "standard:annotation" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new();
        for (i,l) in s.lines().enumerate() {
            let t=l.trim();
            if t.starts_with("@file:") { continue; }
            if t.starts_with('@') && !t.contains("Suppress") && !t.contains("Deprecated") {
                if !t.contains('(') && !t.ends_with("annotation") {
                    v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                        message:"Annotation usage should be checked".into(),auto_fixable:false});
                }
            }
        } v
    }
}

pub struct FunctionLiteralRule;
impl Rule for FunctionLiteralRule {
    fn id(&self) -> &'static str { "standard:function-literal" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let l:Vec<&str>=s.lines().collect();
        for (i,ln) in l.iter().enumerate() {
            let t=ln.trim();
            if (t.ends_with("= {") || t.contains("= {")) && t.contains("val ") {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Function literal should be formatted consistently".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct NoUnitReturnRule;
impl Rule for NoUnitReturnRule {
    fn id(&self) -> &'static str { "standard:no-unit-return" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new();
        for (i,l) in s.lines().enumerate() {
            if l.trim()=="return Unit" || l.trim()=="return" {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Redundant 'return Unit' or empty return".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct NoSingleLineBlockCommentRule;
impl Rule for NoSingleLineBlockCommentRule {
    fn id(&self) -> &'static str { "standard:no-single-line-block-comment" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new();
        for (i,l) in s.lines().enumerate() {
            if l.trim().starts_with("/*") && l.trim().ends_with("*/") {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Use // for single-line comments instead of /* */".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct BlankLineBeforeDeclarationRule;
impl Rule for BlankLineBeforeDeclarationRule {
    fn id(&self) -> &'static str { "standard:blank-line-before-declaration" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let l:Vec<&str>=s.lines().collect();
        for i in 1..l.len() {
            let t=l[i].trim();
            if (t.starts_with("fun ")||t.starts_with("class ")||t.starts_with("val ")||t.starts_with("var ")) {
                let prev=l[i-1].trim();
                if !prev.is_empty() && !prev.starts_with("//") && !prev.starts_with("@") {
                    v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                        message:"Blank line required before declaration".into(),auto_fixable:true});
                }
            }
        } v
    }
}

pub struct SpacingAroundAngleBracketsRule;
impl Rule for SpacingAroundAngleBracketsRule {
    fn id(&self) -> &'static str { "standard:spacing-around-angle-brackets" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let bytes=s.as_bytes();
        for (i,l) in s.lines().enumerate() {
            let t=l.trim();
            if t.contains("< ") && !t.contains("<<") && !t.contains("\"") {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"No space after \"<\" in type arguments".into(),auto_fixable:true});
            }
            if t.contains(" >") && !t.contains(">>") && !t.contains("->") && !t.contains("\"") {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"No space before \">\" in type arguments".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct SpacingAroundUnaryOperatorRule;
impl Rule for SpacingAroundUnaryOperatorRule {
    fn id(&self) -> &'static str { "standard:spacing-around-unary-operator" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new();
        for (i,l) in s.lines().enumerate() {
            if l.contains("! ") && !l.contains("!!") && !l.contains("\"") {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"No space after unary \"!\"".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct FunKeywordSpacingRule;
impl Rule for FunKeywordSpacingRule {
    fn id(&self) -> &'static str { "standard:fun-keyword-spacing" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new();
        for (i,l) in s.lines().enumerate() {
            let t=l.trim();
            if t.starts_with("fun") && (t.starts_with("fun(") || t.contains("fun  ")) {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Missing space after \"fun\" keyword".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct PackageImportSpacingRule;
impl Rule for PackageImportSpacingRule {
    fn id(&self) -> &'static str { "standard:package-import-spacing" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let l:Vec<&str>=s.lines().collect();
        let mut saw_package=false; let mut saw_import=false;
        for (i,ln) in l.iter().enumerate() {
            let t=ln.trim();
            if t.starts_with("package ") { saw_package=true; }
            if t.starts_with("import ") { if saw_package && !saw_import { saw_import=true; } }
            if saw_import && t.is_empty() && i+1<l.len() && l[i+1].trim().starts_with("import ") {
                v.push(Violation{file:String::new(),line:i+2,col:1,rule_id:self.id().into(),
                    message:"No blank line between package and imports".into(),auto_fixable:true});
            }
        } v
    }
}

pub struct MixedConditionOperatorsRule;
impl Rule for MixedConditionOperatorsRule {
    fn id(&self) -> &'static str { "standard:mixed-condition-operators" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new();
        for (i,l) in s.lines().enumerate() {
            if l.contains("&&") && l.contains("||") {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Mixed && and || without parentheses".into(),auto_fixable:false});
            }
        } v
    }
}
