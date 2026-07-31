//! `standard:backing-property-naming` parity with ktlint 1.8.

use crate::rules::{Rule, Violation};

pub struct BackingPropertyNaming;

#[derive(Debug)]
struct Property<'a> {
    name: &'a str,
    column: usize,
    private: bool,
    overridden: bool,
    line: usize,
}

impl Rule for BackingPropertyNaming {
    fn id(&self) -> &'static str {
        "standard:backing-property-naming"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let properties: Vec<Property<'_>> = source
            .lines()
            .enumerate()
            .filter_map(|(line, text)| parse_property(text, line + 1))
            .collect();
        let functions: Vec<&str> = source.lines().filter_map(parse_zero_arg_function).collect();
        let mut violations = Vec::new();

        for property in properties.iter().filter(|property| {
            property.name.starts_with('_') && property.name.len() > 1 && !property.overridden
        }) {
            if !is_lower_camel_backing_name(property.name) {
                violations.push(violation(
                    self.id(),
                    property,
                    "Backing property should start with underscore followed by lower camel case",
                ));
            }

            if !property.private {
                violations.push(violation(
                    self.id(),
                    property,
                    "Backing property not allowed when 'private' modifier is missing",
                ));
            }

            let correlated_name = property.name.trim_start_matches('_');
            let getter_name = format!(
                "get{}{}",
                correlated_name
                    .chars()
                    .next()
                    .map(char::to_uppercase)
                    .into_iter()
                    .flatten()
                    .collect::<String>(),
                correlated_name.chars().skip(1).collect::<String>()
            );
            let correlated = properties
                .iter()
                .any(|candidate| candidate.name == correlated_name)
                || functions.iter().any(|function| *function == getter_name);
            if !correlated {
                violations.push(violation(
                    self.id(),
                    property,
                    "Backing property is only allowed when a matching property or function exists",
                ));
            }
        }
        violations
    }
}

fn violation(rule_id: &str, property: &Property<'_>, message: &str) -> Violation {
    Violation {
        file: String::new(),
        line: property.line,
        col: property.column,
        rule_id: rule_id.to_string(),
        message: message.to_string(),
        auto_fixable: false,
    }
}

fn parse_property(line: &str, line_number: usize) -> Option<Property<'_>> {
    let keyword = ["val", "var"].into_iter().find_map(|keyword| {
        line.match_indices(keyword).find_map(|(index, _)| {
            let before = line[..index].chars().next_back();
            let after = line[index + keyword.len()..].chars().next();
            (before.is_none_or(char::is_whitespace) && after.is_some_and(char::is_whitespace))
                .then_some((index, keyword.len()))
        })
    })?;
    let name_start = keyword.0
        + keyword.1
        + line[keyword.0 + keyword.1..].find(|character: char| !character.is_whitespace())?;
    let name_end = line[name_start..]
        .find(|character: char| !(character == '_' || character.is_alphanumeric()))
        .map_or(line.len(), |offset| name_start + offset);
    let name = &line[name_start..name_end];
    (!name.is_empty()).then(|| Property {
        name,
        column: name_start + 1,
        private: line[..keyword.0]
            .split_whitespace()
            .any(|token| token == "private"),
        overridden: line[..keyword.0]
            .split_whitespace()
            .any(|token| token == "override"),
        line: line_number,
    })
}

fn parse_zero_arg_function(line: &str) -> Option<&str> {
    let fun = line.find("fun ")?;
    let name_start = fun + 4;
    let opening = line[name_start..].find('(')? + name_start;
    let closing = line[opening..].find(')')? + opening;
    (line[opening + 1..closing].trim().is_empty()).then_some(line[name_start..opening].trim())
}

fn is_lower_camel_backing_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters.next() == Some('_')
        && characters.next().is_some_and(char::is_lowercase)
        && characters.all(char::is_alphanumeric)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        BackingPropertyNaming.check(&parser.parse(source), source)
    }

    #[test]
    fn accepts_private_backing_property_with_correlated_property() {
        let source =
            "class Foo {\n    private val _items = listOf<String>()\n    val items = _items\n}\n";
        assert!(check(source).is_empty());
    }

    #[test]
    fn reports_missing_private_modifier_at_identifier() {
        let source = "class Foo {\n    val _items = listOf<String>()\n    val items = _items\n}\n";
        let violations = check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!((violations[0].line, violations[0].col), (2, 9));
        assert_eq!(
            violations[0].message,
            "Backing property not allowed when 'private' modifier is missing"
        );
    }

    #[test]
    fn reports_missing_correlation() {
        let source = "class Foo {\n    private val _orphan = 1\n}\n";
        let violations = check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].col, 17);
        assert_eq!(
            violations[0].message,
            "Backing property is only allowed when a matching property or function exists"
        );
    }
}
