//! standard:no-empty-class-body — flag empty class/interface bodies.
use crate::rules::{Rule, Violation};

pub struct NoEmptyClassBody;

impl Rule for NoEmptyClassBody {
    fn id(&self) -> &'static str {
        "standard:no-empty-class-body"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if (trimmed.ends_with("{}") || trimmed.ends_with("{ }"))
                && (trimmed.contains("class ")
                    || trimmed.contains("interface ")
                    || trimmed.contains("object "))
            {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: self.id().to_string(),
                    message: "Empty class body is unnecessary".to_string(),
                    auto_fixable: false,
                });
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        let t = p.parse(s);
        NoEmptyClassBody.check(&t, s)
    }
    #[test]
    fn non_empty_class() {
        assert!(check("class Foo { val x = 1 }\n").is_empty());
    }
    #[test]
    fn empty_class() {
        let v = check("class Foo {}\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:no-empty-class-body");
    }
}
