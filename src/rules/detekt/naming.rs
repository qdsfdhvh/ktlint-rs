//! detekt naming rules.
use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

// FunctionMaxLength, FunctionMinLength, EnumNaming, FunctionParameterNaming

fn check_fn_name_len(tree: &Tree, source: &str, threshold: usize, is_min: bool) -> Vec<Violation> {
    let mut v = Vec::new();
    let bytes = source.as_bytes();
    fn walk(n: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>, t: usize, imin: bool) {
        if n.kind() == "function_declaration" {
            for i in 0..n.child_count() {
                if let Some(c) = n.child(i) {
                    if c.kind() == "simple_identifier" {
                        if let Ok(name) = c.utf8_text(bytes) {
                            let len = name.chars().count();
                            if (imin && len < t) || (!imin && len > t) {
                                let pos = c.start_position();
                                let rule = if imin {"FunctionMinLength"} else {"FunctionMaxLength"};
                                v.push(Violation{file:String::new(),line:pos.row+1,col:pos.column+1,
                                    rule_id:format!("detekt:naming:{}",rule),
                                    message:if imin{format!("Name \"{}\" too short ({}, min {})",name,len,t)}
                                    else{format!("Name \"{}\" too long ({}, max {})",name,len,t)},
                                    auto_fixable:false});
                            }
                        }
                        break;
                    }
                }
            }
        }
        for i in 0..n.child_count() { if let Some(c)=n.child(i){walk(c,bytes,v,t,imin);} }
    }
    walk(tree.root_node(), bytes, &mut v, threshold, is_min);
    v
}

pub struct FunctionMaxLength { max: usize }
impl FunctionMaxLength { pub fn new()->Self{Self{max:40}} }
impl Rule for FunctionMaxLength { fn id(&self)->&'static str{"detekt:naming:FunctionMaxLength"} fn auto_fixable(&self)->bool{false}
    fn check(&self,t:&Tree,s:&str)->Vec<Violation>{check_fn_name_len(t,s,self.max,false)} }

pub struct FunctionMinLength { min: usize }
impl FunctionMinLength { pub fn new()->Self{Self{min:3}} }
impl Rule for FunctionMinLength { fn id(&self)->&'static str{"detekt:naming:FunctionMinLength"} fn auto_fixable(&self)->bool{false}
    fn check(&self,t:&Tree,s:&str)->Vec<Violation>{check_fn_name_len(t,s,self.min,true)} }

pub struct EnumNaming;
impl Rule for EnumNaming { fn id(&self)->&'static str{"detekt:naming:EnumNaming"} fn auto_fixable(&self)->bool{false}
    fn check(&self,t:&Tree,s:&str)->Vec<Violation>{
        let mut v=Vec::new();let b=s.as_bytes();
        fn w(n:tree_sitter::Node,b:&[u8],v:&mut Vec<Violation>){
            if n.kind()=="enum_entry"{for i in 0..n.child_count(){if let Some(c)=n.child(i){if c.kind()=="simple_identifier"{
                if let Ok(nm)=c.utf8_text(b){if !nm.chars().all(|c|c.is_uppercase()||c.is_ascii_digit()||c=='_'){
                    let p=c.start_position();v.push(Violation{file:String::new(),line:p.row+1,col:p.column+1,
                        rule_id:"detekt:naming:EnumNaming".into(),
                        message:format!("Enum entry \"{}\" should be UPPER_SNAKE_CASE",nm),auto_fixable:false});}}break;}}}}
            for i in 0..n.child_count(){if let Some(c)=n.child(i){w(c,b,v);}}
        }
        w(t.root_node(),b,&mut v);v
    }
}

pub struct FunctionParameterNaming;
impl Rule for FunctionParameterNaming { fn id(&self)->&'static str{"detekt:naming:FunctionParameterNaming"} fn auto_fixable(&self)->bool{false}
    fn check(&self,t:&Tree,s:&str)->Vec<Violation>{
        let mut v=Vec::new();let b=s.as_bytes();
        fn w(n:tree_sitter::Node,b:&[u8],v:&mut Vec<Violation>){
            if n.kind()=="parameter"||n.kind()=="class_parameter"{for i in 0..n.child_count(){
                if let Some(c)=n.child(i){if c.kind()=="simple_identifier"{if let Ok(nm)=c.utf8_text(b){
                    if nm.chars().next().map_or(false,|c|c.is_uppercase()){let p=c.start_position();
                        v.push(Violation{file:String::new(),line:p.row+1,col:p.column+1,
                            rule_id:"detekt:naming:FunctionParameterNaming".into(),
                            message:format!("Parameter \"{}\" should be camelCase",nm),auto_fixable:false});}}break;}}}}
            for i in 0..n.child_count(){if let Some(c)=n.child(i){w(c,b,v);}}
        }
        w(t.root_node(),b,&mut v);v
    }
}

#[cfg(test)] mod tests { use super::*;use crate::parser::KotlinParser;
    fn c(r:&dyn Rule,s:&str)->Vec<Violation>{r.check(&KotlinParser::new().parse(s),s)}
    #[test]fn fn_max_ok(){assert!(c(&FunctionMaxLength::new(),"fun abc(){}\n").is_empty());}
    #[test]fn fn_max_bad(){assert!(!c(&FunctionMaxLength{max:5},"fun abcdef(){}\n").is_empty());}
    #[test]fn fn_min_ok(){assert!(c(&FunctionMinLength::new(),"fun abc(){}\n").is_empty());}
    #[test]fn fn_min_bad(){assert!(!c(&FunctionMinLength{min:5},"fun ab(){}\n").is_empty());}
    #[test]fn enum_ok(){assert!(c(&EnumNaming,"enum class E{FOO}\n").is_empty());}
    #[test]fn enum_bad(){assert!(!c(&EnumNaming,"enum class E{foo}\n").is_empty());}
    #[test]fn param_ok(){assert!(c(&FunctionParameterNaming,"fun f(x:Int)\n").is_empty());}
    #[test]fn param_bad(){assert!(!c(&FunctionParameterNaming,"fun f(X:Int)\n").is_empty());}
}
