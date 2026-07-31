//! `standard:no-empty-class-body` parity with ktlint 1.8.

use crate::rules::{Rule, Violation};

pub struct NoEmptyClassBody;

impl Rule for NoEmptyClassBody {
    fn id(&self) -> &'static str {
        "standard:no-empty-class-body"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(line_index, line)| {
                let opening = empty_declaration_body(line)?;
                Some(Violation {
                    file: String::new(),
                    line: line_index + 1,
                    col: opening + 1,
                    rule_id: self.id().to_string(),
                    message: "Unnecessary block (\"{}\")".to_string(),
                    auto_fixable: true,
                })
            })
            .collect()
    }
}

pub(crate) fn empty_declaration_body(line: &str) -> Option<usize> {
    let code = line.trim_start();
    if code.starts_with("companion object") || code.contains("object :") {
        return None;
    }
    let declaration = code.starts_with("class ")
        || code.starts_with("data class ")
        || code.starts_with("enum class ")
        || code.starts_with("sealed class ")
        || code.starts_with("interface ")
        || code.starts_with("object ");
    if !declaration {
        return None;
    }
    let opening = line.rfind('{')?;
    let closing = line.rfind('}')?;
    (opening < closing && line[opening + 1..closing].trim().is_empty()).then_some(opening)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        NoEmptyClassBody.check(&parser.parse(source), source)
    }

    #[test]
    fn accepts_non_empty_class() {
        assert!(check("class Foo { val value = 1 }\n").is_empty());
    }

    #[test]
    fn reports_opening_brace() {
        let violations = check("class Foo {}\n");
        assert_eq!(violations.len(), 1);
        assert_eq!((violations[0].line, violations[0].col), (1, 11));
        assert_eq!(violations[0].message, "Unnecessary block (\"{}\")");
        assert!(violations[0].auto_fixable);
    }

    #[test]
    fn ignores_empty_companion_object() {
        assert!(check("class Foo {\n    companion object {}\n}\n").is_empty());
    }
}
