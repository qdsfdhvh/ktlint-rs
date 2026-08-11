//! standard:filename — Kotlin file names should match the top-level class
//! (or a lone top-level object). Mirrors ktlint 1.8:
//! - a file whose only non-private top-level declaration is a class /
//!   interface / enum / annotation class must be named after it;
//! - a lone non-private top-level *object* must also match (different
//!   message: "contains a single top level declaration");
//! - private declarations are ignored; two or more public declarations (or
//!   a public class plus a top-level fun/val) disable the check.

use crate::rules::{Rule, Violation};
use std::path::Path;

pub struct Filename;

impl Rule for Filename {
    fn id(&self) -> &'static str {
        "standard:filename"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        self.check_with_path("", tree, source)
    }

    fn check_with_path(
        &self,
        path: &str,
        tree: &tree_sitter::Tree,
        source: &str,
    ) -> Vec<Violation> {
        let Some(file_name) = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| !n.is_empty())
        else {
            return Vec::new();
        };
        // ktlint prints the full file name in the message; matching accepts
        // a platform suffix (`OkHttp.jvm.kt` matches class `OkHttp`).

        // Top-level declarations, private ones ignored.
        let mut public: Vec<tree_sitter::Node> = Vec::new();
        let root = tree.root_node();
        let mut w = root.walk();
        for child in root.children(&mut w) {
            match child.kind() {
                "class_declaration"
                | "object_declaration"
                | "function_declaration"
                | "property_declaration" => {
                    if !is_ignored(&child, source) {
                        public.push(child);
                    }
                }
                _ => {}
            }
        }
        let has_type = public
            .iter()
            .any(|n| matches!(n.kind(), "class_declaration" | "object_declaration"));
        if !has_type && !has_invalid_top_level(&tree, source) {
            // Class-less file (top-level functions/properties only): ktlint
            // still demands the file name conform PascalCase (issue #202
            // follow-up — `for-header-chained.kt`). Invalid files (a bare
            // top-level call expression, parse errors) are reported as
            // parse errors, not naming issues.
            let stem = file_name.trim_end_matches(".kt");
            let pascal_case = stem.chars().next().is_some_and(|c| c.is_uppercase())
                && !stem.contains('_')
                && !stem.contains('-');
            if !pascal_case {
                return vec![Violation {
                    file: String::new(),
                    line: 1,
                    col: 1,
                    rule_id: self.id().to_string(),
                    message: format!("File name '{}' should conform PascalCase", file_name),
                    auto_fixable: false,
                }];
            }
            return Vec::new();
        }
        if public.len() != 1 {
            if has_invalid_top_level(&tree, source) {
                return Vec::new();
            }
            // Multiple public declarations: the file name must still conform
            // PascalCase (oracle: `class-plus-fun.kt`, `two-public-classes.kt`
            // report "File name should conform PascalCase").
            let stem = file_name.rsplit('.').nth(1).unwrap_or(file_name);
            let pascal_case = stem.chars().next().is_some_and(|c| c.is_uppercase())
                && !stem.contains('_')
                && !stem.contains('-');
            if !pascal_case {
                return vec![Violation {
                    file: String::new(),
                    line: 1,
                    col: 1,
                    rule_id: self.id().to_string(),
                    message: format!("File name '{}' should conform PascalCase", file_name),
                    auto_fixable: false,
                }];
            }
            return Vec::new();
        }
        let only = public[0];
        let name = type_name(&only, source);
        let Some(name) = name else { return Vec::new() };
        let matches =
            file_name == format!("{name}.kt") || file_name.starts_with(&format!("{name}."));
        if matches {
            return Vec::new();
        }
        let message = if only.kind() == "object_declaration" {
            format!(
                "File '{}' contains a single top level declaration and should be named '{}.kt'",
                file_name, name
            )
        } else {
            format!(
                "File '{}' contains a single class, and possibly related top level declarations for that class. The file should be named after the class, '{}.kt'",
                file_name, name
            )
        };
        vec![Violation {
            file: String::new(),
            line: 1,
            col: 1,
            rule_id: self.id().to_string(),
            message,
            auto_fixable: false,
        }]
    }
}

/// True when the declaration is excluded from the filename convention
/// (`private` declarations don't count as the file's public class).

/// True when the file carries top-level code that is not a declaration
/// (a bare call expression, an ERROR node) — ktlint reports it as "Not a
/// valid Kotlin file" and the filename convention does not apply.
fn has_invalid_top_level(tree: &tree_sitter::Tree, source: &str) -> bool {
    if tree.root_node().has_error() {
        return true;
    }
    let mut w = tree.root_node().walk();
    for child in tree.root_node().children(&mut w) {
        match child.kind() {
            "package_header" | "comment" | "multiline_comment" | "line_comment" => {}
            "class_declaration"
            | "object_declaration"
            | "function_declaration"
            | "property_declaration" => {}
            "import_header" => {}
            _ => {
                // any other top-level node — e.g. a bare call_expression —
                // is not valid Kotlin at the top level.
                if child.kind() != "ERROR" || child.utf8_text(source.as_bytes()).is_ok() {
                    return true;
                }
            }
        }
    }
    false
}

fn is_ignored(node: &tree_sitter::Node, source: &str) -> bool {
    node.children(&mut node.walk()).any(|c| {
        c.kind() == "modifiers"
            && c.children(&mut c.walk()).any(|m| {
                m.kind() == "visibility_modifier"
                    && m.utf8_text(source.as_bytes()).is_ok_and(|t| t == "private")
            })
    })
}

/// The declared name of a class/interface/enum/annotation/object.
fn type_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    node.children(&mut node.walk())
        .find(|c| c.kind() == "type_identifier")
        .and_then(|c| c.utf8_text(source.as_bytes()).ok())
        .map(|t| t.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check_path(path: &str, source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        Filename.check_with_path(path, &tree, source)
    }

    #[test]
    fn matching_name_is_clean() {
        assert!(check_path("Foo.kt", "class Foo\n").is_empty());
    }

    #[test]
    fn mismatched_class_name_reports() {
        let v = check_path("Bar.kt", "class Foo\n");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "standard:filename");
        assert_eq!(v[0].line, 1);
        assert_eq!(v[0].col, 1);
        assert!(v[0]
            .message
            .contains("should be named after the class, 'Foo.kt'"));
    }

    #[test]
    fn platform_suffix_name_is_clean() {
        // OkHttp.jvm.kt matches class OkHttp.
        assert!(check_path("OkHttp.jvm.kt", "actual object OkHttp\n").is_empty());
    }

    #[test]
    fn lone_object_reports_object_message() {
        let v = check_path("Obj.kt", "object MyObject\n");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("single top level declaration"));
        assert!(v[0].message.contains("'MyObject.kt'"));
    }

    #[test]
    fn private_class_is_ignored() {
        assert!(check_path("Any.kt", "private class Hidden\n").is_empty());
    }

    #[test]
    fn two_public_classes_disable_check() {
        assert!(check_path("Any.kt", "class First\n\nclass Second\n").is_empty());
    }

    #[test]
    fn class_plus_top_level_fun_disables_check() {
        assert!(check_path("Any.kt", "class Main\n\nfun helper() = 1\n").is_empty());
    }

    #[test]
    fn public_class_with_private_helper_reports() {
        let v = check_path("Any.kt", "class Main\n\nprivate class Helper\n");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("'Main.kt'"));
    }
}
