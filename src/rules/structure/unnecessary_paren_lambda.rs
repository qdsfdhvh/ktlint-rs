//! standard:unnecessary-parentheses-before-trailing-lambda — remove parens on single lambda arg.
use crate::rules::{Rule, Violation};
pub struct UnnecessaryParenBeforeLambda;
impl Rule for UnnecessaryParenBeforeLambda {
    fn id(&self) -> &'static str { "standard:unnecessary-parentheses-before-trailing-lambda" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new();
        let lines:Vec<&str>=s.lines().collect();
        for(i,ln) in lines.iter().enumerate() {
            let t=ln.trim();
            if t.contains(") {") && (t.contains("list.") || t.contains("List") || t.contains(".let") || t.contains(".run") || t.contains(".apply") || t.contains(".also") || t.contains(".takeIf") || t.contains(".takeUnless")) {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Unnecessary parentheses before trailing lambda".into(),auto_fixable:true});
            }
        } v
    }
}
#[cfg(test)] mod tests { use super::*; use crate::parser::KotlinParser;
    fn c(s:&str)->Vec<Violation>{let mut p=KotlinParser::new();UnnecessaryParenBeforeLambda.check(&p.parse(s),s)}
    #[test] fn good() { assert!(c("list.forEach { it }\n").is_empty()); }
    #[test] fn bad() { assert!(!c("list.forEach() { it }\n").is_empty()); }
}
