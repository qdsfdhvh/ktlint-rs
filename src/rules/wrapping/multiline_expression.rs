//! standard:multiline-expression-wrapping — JVM ktlint parity.
//! Catches multi-line expressions that should be wrapped consistently.
use crate::rules::{Rule, Violation};

pub struct MultilineExpressionWrapping;
impl Rule for MultilineExpressionWrapping {
    fn id(&self) -> &'static str { "standard:multiline-expression-wrapping" }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let l:Vec<&str>=s.lines().collect();
        let mut in_when = false; let mut when_open_line = 0usize;
        for (i,ln) in l.iter().enumerate() {
            let t=ln.trim();
            if t.starts_with("when ") || t.starts_with("when(") {
                in_when = true; when_open_line = i;
            }
            if in_when && t == "}" { in_when = false; }
            // Catch when entries that span multiple lines inconsistently
            if in_when && t.contains("->") {
                let after = t.split("->").nth(1).unwrap_or("").trim();
                if after.is_empty() && i+1 < l.len() && !l[i+1].trim().is_empty() && !l[i+1].trim().starts_with("{") && l[i+1].trim() != "}" {
                    v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                        message:"Multi-line when entry should be wrapped consistently".into(),auto_fixable:true});
                }
            }
            // Catch condition chains that should wrap
            if t.contains("&&") && t.contains("=>") {
                v.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Multi-line expression should be wrapped for readability".into(),auto_fixable:true});
            }
        }
        v
    }
}
