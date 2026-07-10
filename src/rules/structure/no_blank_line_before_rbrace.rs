//! standard:no-blank-line-before-rbrace — no blank lines immediately before closing }.
use crate::rules::{Rule, Violation};
pub struct NoBlankLineBeforeRbrace;
impl Rule for NoBlankLineBeforeRbrace {
    fn id(&self) -> &'static str { "standard:no-blank-line-before-rbrace" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let l:Vec<&str>=s.lines().collect();
        for (i,ln) in l.iter().enumerate() {
            if ln.trim()=="}" && i>0 && l[i-1].trim().is_empty() {
                // Skip if 2 lines back is also } (nested closing braces with blank)
                if i>=2 && l[i-2].trim()=="}" { continue; }
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Blank line(s) before \"}\"".into(),auto_fixable:true});
            }
        } v
    }
}
#[cfg(test)] mod tests { use super::*; use crate::parser::KotlinParser;
    fn c(s:&str)->Vec<Violation>{let mut p=KotlinParser::new();NoBlankLineBeforeRbrace.check(&p.parse(s),s)}
    #[test] fn ok(){assert!(c("fun f(){\n    val x=1\n}\n").is_empty());}
    #[test] fn bad(){assert!(!c("fun f(){\n\n}\n").is_empty());}
}
