//! detekt:style:UseOrEmpty — x ?: run { emptyList() } should be x.orEmpty()
use crate::rules::{Rule, Violation};

pub struct UseOrEmpty;

impl Rule for UseOrEmpty {
    fn id(&self) -> &'static str {
        "detekt:style:UseOrEmpty"
    }
    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, line)| {
                let t = line.trim();
                if (t.contains("?: emptyList()")
                    || t.contains("?: listOf()")
                    || t.contains("?: emptySet()")
                    || t.contains("?: setOf()")
                    || t.contains("?: emptyMap()")
                    || t.contains("?: mapOf()")
                    || t.contains("?: emptySequence()"))
                    && !t.contains("orEmpty()")
                {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:style:UseOrEmpty".into(),
                        message: "Use .orEmpty() instead of ?: emptyList()".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        UseOrEmpty.check(&KotlinParser::new().parse(s), s)
    }
    #[test]
    fn elvis_empty_list_bad() {
        assert!(!c("val x = list ?: emptyList()\n").is_empty());
    }
    #[test]
    fn or_empty_ok() {
        assert!(c("val x = list.orEmpty()\n").is_empty());
    }
}
