//! standard:no-empty-line-after-kdoc — no blank line between KDoc and declaration.
use crate::rules::{Rule, Violation};
pub struct NoBlankAfterKdoc;
impl Rule for NoBlankAfterKdoc {
    fn id(&self) -> &'static str {
        "standard:no-empty-line-after-kdoc"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            if ln.trim().ends_with("*/") && i + 1 < l.len() && l[i + 1].trim().is_empty() {
                // Only flag if the line after the blank line is NOT a declaration,
                // annotation, or modifier (standard KDoc style allows blank line before decl).
                if i + 2 < l.len() {
                    let next = l[i + 2].trim();
                    // Skip annotations, modifiers, and declarations
                    let is_decl_or_related = next.starts_with('@')
                        || next.starts_with("class ")
                        || next.starts_with("fun ")
                        || next.starts_with("val ")
                        || next.starts_with("var ")
                        || next.starts_with("object ")
                        || next.starts_with("interface ")
                        || next.starts_with("enum ")
                        || next.starts_with("data class ")
                        || next.starts_with("sealed ")
                        || next.starts_with("abstract ")
                        || next.starts_with("open ")
                        || next.starts_with("internal ")
                        || next.starts_with("private ")
                        || next.starts_with("protected ")
                        || next.starts_with("public ")
                        || next.starts_with("suspend ")
                        || next.starts_with("operator ")
                        || next.starts_with("inline ")
                        || next.starts_with("tailrec ")
                        || next.starts_with("external ")
                        || next.starts_with("expect ")
                        || next.starts_with("actual ")
                        || next.starts_with("const ")
                        || next.starts_with("override ")
                        || next == "companion"
                        || next.starts_with("companion ");
                    if is_decl_or_related {
                        continue;
                    }
                }
                v.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().into(),
                    message: "Unexpected blank line after KDoc comment".into(),
                    auto_fixable: true,
                });
            }
        }
        v
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        NoBlankAfterKdoc.check(&p.parse(s), s)
    }
    #[test]
    fn ok() {
        assert!(c("/** doc */\nclass Foo\n").is_empty());
    }
    #[test]
    fn blank_before_declaration_is_ok() {
        // Standard KDoc style: blank line between KDoc and class/fun/etc is fine
        assert!(c("/** doc */\n\nclass Foo\n").is_empty());
        assert!(c("/** doc */\n\nfun bar() {}\n").is_empty());
        assert!(c("/** doc */\n\nval x = 1\n").is_empty());
        assert!(c("/** doc */\n\n@Annotation\nclass Foo\n").is_empty());
    }

    #[test]
    fn blank_before_non_declaration_is_bad() {
        assert!(!c("/** doc */\n\nprintln(\"x\")\n").is_empty());
    }
}
