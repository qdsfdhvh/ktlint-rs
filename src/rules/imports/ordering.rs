//! standard:import-ordering — imports sorted alphabetically, grouped by package.

use crate::rules::{Rule, Violation};

pub struct ImportOrdering;

impl Rule for ImportOrdering {
    fn id(&self) -> &'static str {
        "standard:import-ordering"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let imports: Vec<(usize, &str)> = source
            .lines()
            .enumerate()
            .filter(|(_, line)| line.trim().starts_with("import "))
            .map(|(i, line)| (i, line.trim_start()))
            .collect();

        if imports.len() < 2 {
            return violations;
        }

        // Check that imports are ordered alphabetically
        // Group by top-level package prefix (before the first `.` if any)
        let lines: Vec<String> = imports.iter().map(|(_, l)| l.to_string()).collect();
        let mut sorted = lines.clone();
        sorted.sort();

        if lines != sorted {
            // Find first violation
            for i in 0..lines.len() {
                if lines[i] != sorted[i] {
                    violations.push(Violation {
                        file: String::new(),
                        line: imports[i].0 + 1,
                        col: 1,
                        rule_id: self.id().to_string(),
                        message: format!("Import \"{}\" is not in alphabetical order", lines[i]),
                        auto_fixable: true,
                    });
                    break;
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

    fn check(source: &str) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        ImportOrdering.check(&tree, source)
    }

    #[test]
    fn sorted_imports() {
        assert!(check(
            "package foo\n\nimport android.view.View\nimport java.io.File\n\nclass Bar\n"
        )
        .is_empty());
    }

    #[test]
    fn unsorted_imports() {
        let v =
            check("package foo\n\nimport java.io.File\nimport android.view.View\n\nclass Bar\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:import-ordering");
    }
}
