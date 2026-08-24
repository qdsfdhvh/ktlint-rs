//! `standard:class-naming` parity with ktlint 1.8.

use crate::rules::{Rule, Violation};

pub struct ClassNaming;

impl Rule for ClassNaming {
    fn id(&self) -> &'static str {
        "standard:class-naming"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let junit_file = source.lines().any(|line| {
            line.trim_start()
                .starts_with("import org.junit.jupiter.api")
        });
        // Issue #260: `class`/`interface`/`object` appearing inside a string
        // literal (e.g. "Expected a JSON object at the root") is prose, not
        // a declaration. Collect the byte ranges of string-literal CST nodes
        // and ignore keyword matches that land inside them.
        let mut string_ranges: Vec<(usize, usize)> = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind().contains("string") {
                string_ranges.push((node.start_byte(), node.end_byte()));
            }
            let mut w = node.walk();
            for c in node.children(&mut w) {
                stack.push(c);
            }
        }
        let mut violations = Vec::new();
        let line_offsets: Vec<usize> = {
            let mut off = 0usize;
            source
                .lines()
                .map(|l| {
                    let o = off;
                    off += l.len() + 1;
                    o
                })
                .collect()
        };
        for (line_index, line) in source.lines().enumerate() {
            // Skip comment/KDoc lines: `class`/`interface`/`object` inside
            // prose (e.g. "The class of bug...") is not a declaration.
            let trimmed_line = line.trim_start();
            if trimmed_line.starts_with("//")
                || trimmed_line.starts_with("/*")
                || trimmed_line.starts_with("/**")
                || trimmed_line.starts_with("*")
            {
                continue;
            }
            let declaration = ["class ", "interface ", "object "]
                .into_iter()
                .filter_map(|keyword| {
                    line.find(keyword).and_then(|start| {
                        let abs = line_offsets[line_index] + start;
                        let in_string = string_ranges.iter().any(|&(s, e)| s <= abs && abs < e);
                        if in_string {
                            None
                        } else {
                            Some((start, keyword))
                        }
                    })
                })
                .min_by_key(|(start, _)| *start);
            let Some((keyword_start, keyword)) = declaration else {
                continue;
            };
            let name_start = keyword_start + keyword.len();
            let Some((name, display_name)) = declaration_name(&line[name_start..]) else {
                continue;
            };
            let is_data_object =
                keyword == "object " && line[..keyword_start].trim_end().ends_with("data");
            let upper_snake = !name.is_empty()
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_');
            if valid_name(name)
                || (is_data_object && upper_snake)
                || (display_name.starts_with('`')
                    && display_name.ends_with('`')
                    && (junit_file || is_keyword(name)))
            {
                continue;
            }
            violations.push(Violation {
                file: String::new(),
                line: line_index + 1,
                col: name_start + 1,
                rule_id: self.id().to_string(),
                message:
                    "Class or object name should start with an uppercase letter and use camel case"
                        .to_string(),
                auto_fixable: false,
            });
        }
        violations
    }
}

fn declaration_name(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();
    if let Some(rest) = input.strip_prefix('`') {
        let end = rest.find('`')? + 2;
        return Some((&input[1..end - 1], &input[..end]));
    }
    let end = input
        .find(|character: char| !(character == '_' || character.is_alphanumeric()))
        .unwrap_or(input.len());
    (end > 0).then_some((&input[..end], &input[..end]))
}

fn valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters.next().is_some_and(char::is_uppercase) && characters.all(char::is_alphanumeric)
}

fn is_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "class"
            | "continue"
            | "do"
            | "else"
            | "false"
            | "for"
            | "fun"
            | "if"
            | "in"
            | "interface"
            | "is"
            | "null"
            | "object"
            | "package"
            | "return"
            | "super"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typealias"
            | "typeof"
            | "val"
            | "var"
            | "when"
            | "while"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        ClassNaming.check(&parser.parse(source), source)
    }

    #[test]
    fn accepts_pascal_case_class() {
        assert!(check("class MyViewModel1\n").is_empty());
    }

    #[test]
    fn reports_class_at_identifier() {
        let violations = check("class my_view_model\n");
        assert_eq!(violations.len(), 1);
        assert_eq!((violations[0].line, violations[0].col), (1, 7));
        assert_eq!(
            violations[0].message,
            "Class or object name should start with an uppercase letter and use camel case"
        );
    }

    #[test]
    fn reports_object_at_identifier() {
        let violations = check("object invalid_name\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].col, 8);
    }

    // Issue #260: `class`/`interface`/`object` inside a string literal is
    // prose, not a declaration ("Expected a JSON object at the root").
    #[test]
    fn ignores_keywords_inside_string_literals() {
        let src = "package com.example\n\npublic class ClassNaming {\n    public fun describe(kind: Int): String {\n        return when (kind) {\n            0 -> \"Expected a JSON object at the root\"\n            else -> \"unknown\"\n        }\n    }\n}\n";
        assert!(check(src).is_empty());
        // The real declaration still reports normally.
        let v = check("public class bad_name {\n    val x = \"an object in a string\"\n}\n");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 1);
    }
}
