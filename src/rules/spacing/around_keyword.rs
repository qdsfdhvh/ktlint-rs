//! standard:spacing-around-keyword — spaces around if/for/while/try/catch keywords.
use crate::rules::{Rule, Violation};

pub struct SpacingAroundKeyword;

impl Rule for SpacingAroundKeyword {
    fn id(&self) -> &'static str {
        "standard:spacing-around-keyword"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let bytes = source.as_bytes();
        let mut violations = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if matches!(
                node.kind(),
                "if" | "for" | "while" | "try" | "catch" | "when"
            ) {
                let pos = node.start_position();
                if bytes.get(node.end_byte()) == Some(&b'(') {
                    violations.push(Violation {
                        file: String::new(),
                        line: pos.row + 1,
                        col: pos.column + 1,
                        rule_id: self.id().into(),
                        message: format!("Missing space after \"{}\"", node.kind()),
                        auto_fixable: true,
                    });
                }
            }
            for index in (0..node.child_count()).rev() {
                if let Some(child) = node.child(index) {
                    stack.push(child);
                }
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
        SpacingAroundKeyword.check(&t, s)
    }
    #[test]
    fn valid_keyword() {
        assert!(check("if (x) {}\n").is_empty());
    }
    #[test]
    fn missing_space() {
        let v = check("val x=if(true)1 else 2\n");
        assert!(!v.is_empty());
    }
}
