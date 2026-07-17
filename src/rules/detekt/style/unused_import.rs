//! detekt:style:UnusedImport — flags import statements not referenced in file.
//! Uses name resolution engine (L1) for precise detection.

use crate::rules::{Rule, Violation};
use std::collections::HashSet;

pub struct UnusedImport;

impl Rule for UnusedImport {
    fn check(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        {
            use crate::resolver::builder::build_symbol_table;
            let sym = build_symbol_table(source, tree.root_node());
            self.check_with_symbols(tree, source, Some(&sym))
        }
    }

    fn id(&self) -> &'static str {
        "detekt:style:UnusedImport"
    }

    fn check_with_symbols(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        sym: Option<&crate::resolver::SymbolTable>,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();
        let table = sym.expect("SymbolTable should be provided by engine");

        // Collect all referenced names. The skip flag is propagated down the
        // DFS — never walk Node::parent() per identifier: tree-sitter's
        // parent() re-descends from the root each call, which is O(n²·depth)
        // on deeply nested files.
        let used: HashSet<String> = {
            let mut u = HashSet::new();
            let bytes = source.as_bytes();
            let mut stack = vec![(tree.root_node(), false)];
            const SKIP: &[&str] = &["import_header", "package_header"];
            while let Some((node, in_skip)) = stack.pop() {
                let skip = in_skip || SKIP.contains(&node.kind());
                if !skip && (node.kind() == "simple_identifier" || node.kind() == "type_identifier")
                {
                    if let Ok(name) = node.utf8_text(bytes) {
                        u.insert(name.to_string());
                    }
                }
                for i in (0..node.child_count()).rev() {
                    if let Some(c) = node.child(i) {
                        stack.push((c, skip));
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
    #[test]
    fn aliased_import_used_ok() {
        assert!(c("import java.util.UUID as Uid\n\nval id = Uid.randomUUID()\n").is_empty());
    }
}
