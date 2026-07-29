//! standard:parameter-list-spacing — no extra spaces in parameter lists.
use crate::rules::{Rule, Violation};

pub struct ParameterListSpacing;
impl Rule for ParameterListSpacing {
    fn id(&self) -> &'static str {
        "standard:parameter-list-spacing"
    }
    fn auto_fixable(&self) -> bool {
        true
    }
    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains('(') && trimmed.contains(')') {
                if let Some(paren_start) = trimmed.find('(') {
                    let list_context = trimmed[..paren_start]
                        .trim_end()
                        .chars()
                        .last()
                        .is_some_and(|ch| ch.is_alphanumeric() || matches!(ch, '_' | ')' | ']'));
                    if !list_context {
                        continue;
                    }
                    if let Some(paren_end) = trimmed.rfind(')') {
                        if paren_end > paren_start + 1 {
                            let params = &trimmed[paren_start + 1..paren_end];
                            let structural = without_string_contents(params);
                            let has_extra_space = params.starts_with(char::is_whitespace)
                                || params.ends_with(char::is_whitespace)
                                || structural.contains(" ,")
                                || structural.contains(",  ");
                            if has_extra_space {
                                violations.push(Violation {
                                    file: String::new(),
                                    line: i + 1,
                                    col: paren_start + 2,
                                    rule_id: self.id().to_string(),
                                    auto_fixable: true,
                                    message: "Extra spaces in parameter list".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
        violations
    }
}

fn without_string_contents(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut quote = None;
    let mut escaped = false;
    for ch in text.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote.is_some() && ch == '\\' {
            escaped = true;
            continue;
        }
        if matches!(ch, '\'' | '"') {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
                result.push('S');
            }
            continue;
        }
        if quote.is_none() {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        ParameterListSpacing.check(&p.parse(s), s)
    }
    #[test]
    fn ok() {
        assert!(c("fun foo(a: Int, b: String)\n").is_empty());
    }
    #[test]
    fn double() {
        assert!(!c("fun foo( a: Int,  b: String)\n").is_empty());
    }
    #[test]
    fn empty() {
        assert!(c("fun foo()\n").is_empty());
    }

    #[test]
    fn spaces_inside_string_argument_are_allowed() {
        assert!(c("call(\"  padded  \")\n").is_empty());
    }

    #[test]
    fn string_arguments_around_comma_preserve_structural_boundary() {
        assert!(c("call(name = \"value\", description = \"other\")\n").is_empty());
    }
}
