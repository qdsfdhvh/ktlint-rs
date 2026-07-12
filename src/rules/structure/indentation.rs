//! standard:indent — JVM ktlint parity. Checks indent is multiple of indent_size
//! and detects missing indentation inside blocks.
use crate::rules::{Rule, Violation};

pub struct Indentation { indent_size: usize }
impl Indentation {
    pub fn new(indent_size: usize) -> Self { Self { indent_size } }
}
impl Rule for Indentation {
    fn id(&self) -> &'static str { "standard:indent" }
    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let indent_size = self.indent_size;
        let lines: Vec<&str> = source.lines().collect();
        let mut expected_indent = 0usize;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }

            let spaces = line.len() - trimmed.len();

            // Tab indentation
            if line.starts_with('\t') {
                violations.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:"Unexpected tab character(s)".into(),auto_fixable:true});
                continue;
            }

            // If line opens a block, increase expected indent for next line
            if trimmed.ends_with('{') || trimmed == "{" {
                expected_indent += indent_size;
            }
            // If line closes a block, decrease expected indent
            if trimmed.starts_with('}') {
                expected_indent = expected_indent.saturating_sub(indent_size);
            }

            // Check current indent
            if spaces > 0 && spaces % indent_size != 0 {
                violations.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:format!("Unexpected indentation ({}) — should be a multiple of {}", spaces, indent_size),
                    auto_fixable:true});
            } else if spaces == 0 && expected_indent > 0 && !trimmed.starts_with("package ") && !trimmed.starts_with("import ") && !trimmed.starts_with("//") {
                violations.push(Violation{file:String::new(),line:i+1,col:1,rule_id:self.id().into(),
                    message:format!("Unexpected indentation (0) (should be {})", expected_indent),
                    auto_fixable:true});
            }
        }
        violations
    }
}

#[cfg(test)] mod tests { use super::*; use crate::parser::KotlinParser;
    fn c(s:&str)->Vec<Violation>{let mut p=KotlinParser::new();Indentation::new(4).check(&p.parse(s),s)}
    #[test] fn ok(){assert!(c("fun f(){\n    val x=1\n}\n").is_empty());}
    #[test] fn bad(){assert!(!c("fun f(){\n   val x=1\n}\n").is_empty());}
    #[test] fn zero_indent(){assert!(!c("fun f(){\nval x=1\n}\n").is_empty());}
}
