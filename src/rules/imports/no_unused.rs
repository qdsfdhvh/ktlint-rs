//! standard:no-unused-imports — detects unused import statements.

use crate::rules::{Rule, Violation};
use std::collections::HashSet;

pub struct NoUnusedImports;

impl Rule for NoUnusedImports {
    fn id(&self) -> &'static str {
        "standard:no-unused-imports"
    }

    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Collect imports
        let imports: Vec<(usize, String)> = source
            .lines()
            .enumerate()
            .filter(|(_, line)| line.trim().starts_with("import "))
            .map(|(i, line)| (i, Self::extract_import_name(line.trim())))
            .filter(|(_, name)| !name.is_empty())
            .collect();

        if imports.is_empty() {
            return violations;
        }

        // Collect all identifiers used in the code (excluding import lines)
        let used_ids = self.collect_identifiers(tree, source);

        for (line, import_name) in &imports {
            if !used_ids.contains(import_name) {
                violations.push(Violation {
                    file: String::new(),
                    line: line + 1,
                    col: 1,
                    rule_id: self.id().to_string(),
                    message: format!("Unused import \"{}\"", import_name),
                    auto_fixable: false, // removing imports can break things
                });
            }
        }

        violations
    }
}

impl NoUnusedImports {
    fn extract_import_name(line: &str) -> String {
        // "import java.io.File" → "File"
        // "import kotlin.collections.*" → "kotlin.collections.*"
        let content = line
            .trim()
            .trim_start_matches("import ")
            .trim()
            .to_string();

        // Check for wildcard imports — don't flag these as unused
        if content.ends_with(".*") {
            return String::new(); // can't reliably check wildcard usage
        }

        // Get the last segment
        content
            .split('.')
            .last()
            .unwrap_or(&content)
            .to_string()
    }

    fn collect_identifiers(&self, tree: &tree_sitter::Tree, source: &str) -> HashSet<String> {
        let mut ids = HashSet::new();
        self.walk_for_ids(tree.root_node(), source, &mut ids);
        ids
    }

    fn walk_for_ids(&self, node: tree_sitter::Node, source: &str, ids: &mut HashSet<String>) {
        let kind = node.kind();

        // Skip import headers
        if kind == "import_header" || kind == "import_list" {
            return;
        }

        // Collect identifiers
        if kind == "simple_identifier"
            || kind == "type_identifier"
            || kind == "user_type"
        {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                ids.insert(text.to_string());
            }
        }

        // Also collect constructor invocations: `Foo()` → "Foo"
        if kind == "constructor_invocation" {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "user_type" {
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            ids.insert(text.to_string());
                        }
                    }
                }
            }
        }

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk_for_ids(child, source, ids);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        NoUnusedImports.check(&tree, source)
    }

    #[test]
    fn used_import() {
        assert!(check("import java.io.File\n\nval f = File(\"test\")\n").is_empty());
    }

    #[test]
    fn unused_import_detected() {
        let v = check("import java.io.File\n\nval x = 1\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:no-unused-imports");
    }

    #[test]
    fn wildcard_import_not_flagged() {
        assert!(check("import kotlin.collections.*\n\nval x = 1\n").is_empty());
    }
}
