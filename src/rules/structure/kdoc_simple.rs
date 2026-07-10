//! standard:kdoc — focused: only empty KDocs and @param style.
use crate::rules::{Rule, Violation};
pub struct KdocSimple;
impl Rule for KdocSimple {
    fn id(&self) -> &'static str { "standard:kdoc" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let l:Vec<&str>=s.lines().collect();
        for (i,ln) in l.iter().enumerate() {
            let t=ln.trim();
            if t=="/**" && i+1<l.len() && l[i+1].trim()=="*/" {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"KDoc comment must not be empty".into(),auto_fixable:true});
            }
            if t.starts_with("* @param") && !t.contains("[") {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Use KDoc syntax @param[name] instead of @param".into(),auto_fixable:true});
            }
        } v
    }
}
#[cfg(test)] mod tests { use super::*; use crate::parser::KotlinParser;
    fn c(s:&str)->Vec<Violation>{let mut p=KotlinParser::new();KdocSimple.check(&p.parse(s),s)}
    #[test] fn empty(){assert!(!c("/**\n */\n").is_empty());}
    #[test] fn ok(){assert!(c("/** doc */\n").is_empty());}
}
