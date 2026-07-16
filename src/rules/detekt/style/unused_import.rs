//! detekt:style:UnusedImport — flags import statements not referenced in file.
//! Uses name resolution engine (L1) for precise detection.

use crate::resolver::builder::build_symbol_table;
use crate::rules::{Rule, Violation};
use std::collections::HashSet;

pub struct UnusedImport;

impl Rule for UnusedImport {
    fn id(&self) -> &'static str {
        "detekt:style:UnusedImport"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let table = build_symbol_table(source, tree.root_node());

        // Collect all referenced names
        let used: HashSet<String> = {
            let mut u = HashSet::new();
            let bytes = source.as_bytes();
            let mut stack = vec![tree.root_node()];
            const SKIP: &[&str] = &["import_header", "package_header"];
            while let Some(node) = stack.pop() {
                if node.kind() == "simple_identifier" || node.kind() == "type_identifier" {
                    if let Ok(name) = node.utf8_text(bytes) {
                        let mut is_skip = false;
                        let mut cur = node.parent();
                        while let Some(p) = cur {
                            if SKIP.contains(&p.kind()) {
                                is_skip = true;
                                break;
                            }
                            cur = p.parent();
                        }
                        if !is_skip {
                            u.insert(name.to_string());
                        }
                    }
                }
                for i in (0..node.child_count()).rev() {
                    if let Some(c) = node.child(i) {
                        stack.push(c);
                    }
                }
            }
            u
        };

        // Check each import against usage
        for (alias, _full_path) in &table.imports {
            let simple = alias.rsplit('.').next().unwrap_or(alias);
            if !used.contains(simple) && !used.contains(alias) {
                // Find the import line
                for (i, line) in source.lines().enumerate() {
                    let t = line.trim();
                    if t.starts_with("import ") && t.contains(alias.as_str()) {
                        violations.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: "detekt:style:UnusedImport".into(),
                            message: format!("Import '{}' is never used", alias),
                            auto_fixable: true,
                        });
                        break;
                    }
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

    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        UnusedImport.check(&p.parse(s), s)
    }

    #[test]
    fn used_import_ok() {
        assert!(c("import com.Foo\nclass Bar(val x: Foo)\n").is_empty());
    }
    #[test]
    fn unused_import_bad() {
        assert!(!c("import com.Foo\nclass Bar {}\n").is_empty());
    }
}
