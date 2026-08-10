//! standard:import-ordering — imports sorted by the IntelliJ imports layout
//! (case-sensitive ASCII), then lexicographically within each group.
//!
//! Mirrors ktlint 1.8:
//! - `ij_kotlin_imports_layout` (when set) defines the groups; `*` alone
//!   means a single group (pure lexicographic);
//! - without it, `android_studio` sorts purely lexicographically, while
//!   `ktlint_official` / `intellij_idea` use the official group order
//!   `android, androidx, com, kotlinx, *, java, javax, kotlin`
//!   (verified against the ktlint 1.8.0 CLI).

use crate::config::CodeStyle;
use crate::rules::{Rule, Violation};

/// The group order ktlint_official / intellij_idea use when no layout is
/// configured — from the ktlint 1.8.0 bytecode: `*, java.**, javax.**,
/// kotlin.**` (a catch-all `*` group first, then java/javax/kotlin last).
/// So android/androidx/com/kotlinx/org all sort lexicographically inside
/// the `*` group; only java/javax/kotlin sort after everything else.
/// From the ktlint 1.8.0 bytecode: `*, java.**, javax.**, kotlin.**, ^` —
/// the `^` marks the aliases group (sorted last, by alias name).
const OFFICIAL_DEFAULT_LAYOUT: &[&str] = &["*", "java", "javax", "kotlin", "^"];

pub struct ImportOrdering {
    layout: Vec<String>,
}

impl ImportOrdering {
    /// Build from the resolved editorconfig: `ij_kotlin_imports_layout`
    /// (lives in the `ij_kotlin_properties` bucket) or the code style's
    /// default.
    pub fn new(
        code_style: CodeStyle,
        ij_properties: &std::collections::HashMap<String, String>,
    ) -> Self {
        let configured = ij_properties
            .get("ij_kotlin_imports_layout")
            .map(|s| s.as_str());
        let layout = match configured {
            Some(cfg) => parse_layout(cfg),
            None if code_style == CodeStyle::AndroidStudio => vec!["*".to_string()],
            None => OFFICIAL_DEFAULT_LAYOUT
                .iter()
                .map(|g| g.to_string())
                .collect(),
        };
        Self { layout }
    }

    fn group_index(&self, import_path: &str) -> usize {
        // An exact (non-`*`) group match wins; `*` is only a fallback so a
        // catch-all group placed in the middle of the layout does not
        // capture `java`/`javax`/`kotlin` imports.
        if let Some(idx) = self.layout.iter().position(|group| {
            group != "*"
                && import_path.starts_with(group)
                && import_path[group.len()..].starts_with('.')
        }) {
            return idx;
        }
        self.layout
            .iter()
            .position(|group| group == "*")
            .unwrap_or(0)
    }

    /// (group, path, alias): the sort key is the *import path* for every
    /// import (aliased or not) — verified against ktlint 1.8.0 under both
    /// the official layout and `ij_kotlin_imports_layout = *`. The alias
    /// only breaks ties on the same path, and a `^` group in the layout
    /// (official default) moves aliased imports to the end.
    fn sort_key(&self, import: &str) -> (usize, String, String) {
        let alias_group = self.layout.iter().position(|g| g == "^");
        match (import.split_once(" as "), alias_group) {
            (Some((path, alias)), Some(idx)) => {
                (idx, path.trim().to_string(), alias.trim().to_string())
            }
            (Some((path, alias)), None) => (
                self.group_index(path.trim()),
                path.trim().to_string(),
                alias.trim().to_string(),
            ),
            (None, _) => (
                self.group_index(import),
                import.trim().to_string(),
                String::new(),
            ),
        }
    }
}

/// Parse an `ij_kotlin_imports_layout` value: comma-separated groups where
/// `**` / `*` suffixes are ignored (a bare `*` group is the fallback and
/// sorts after named groups only if it appears later in the list).
fn parse_layout(cfg: &str) -> Vec<String> {
    cfg.split(',')
        .map(|g| {
            let g = g.trim();
            if g == "*" {
                return g.to_string();
            }
            g.trim_end_matches("**").trim_end_matches('*').to_string()
        })
        .collect()
}

impl Rule for ImportOrdering {
    fn id(&self) -> &'static str {
        "standard:import-ordering"
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let imports: Vec<(usize, String)> = source
            .lines()
            .enumerate()
            .filter(|(_, line)| line.trim().starts_with("import "))
            .map(|(i, line)| {
                // keep the whole import (incl. any " as X" alias) so the
                // sorter can tell aliased imports apart
                let t = line.trim_start();
                (i, t.trim_start_matches("import ").to_string())
            })
            .collect();

        if imports.len() < 2 {
            return violations;
        }

        let mut sorted = imports.clone();
        sorted.sort_by(|a, b| {
            let ka = self.sort_key(&a.1);
            let kb = self.sort_key(&b.1);
            ka.cmp(&kb)
        });

        for i in 0..imports.len() {
            if imports[i].1 != sorted[i].1 {
                violations.push(Violation {
                    file: String::new(),
                    line: imports[i].0 + 1,
                    col: 1,
                    rule_id: self.id().to_string(),
                    message: format!(
                        "Import \"{}\" is not in alphabetical order",
                        imports[i].1.split(" as ").next().unwrap_or("")
                    ),
                    auto_fixable: true,
                });
                break;
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn check_with(source: &str, style: CodeStyle, layout: Option<&str>) -> Vec<Violation> {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        let props = layout
            .map(|l| {
                let mut m = std::collections::HashMap::new();
                m.insert("ij_kotlin_imports_layout".to_string(), l.to_string());
                m
            })
            .unwrap_or_default();
        ImportOrdering::new(style, &props).check(&tree, source)
    }

    fn official(source: &str) -> Vec<Violation> {
        check_with(source, CodeStyle::KtlintOfficial, None)
    }
    fn android(source: &str) -> Vec<Violation> {
        check_with(source, CodeStyle::AndroidStudio, None)
    }
    fn star(source: &str) -> Vec<Violation> {
        check_with(source, CodeStyle::KtlintOfficial, Some("*"))
    }

    #[test]
    fn sorted_imports() {
        assert!(official(
            "package foo\n\nimport android.view.View\nimport androidx.core.File\nimport com.foo.Bar\nimport java.io.File\n"
        )
        .is_empty());
    }

    #[test]
    fn unsorted_imports() {
        let v =
            official("package foo\n\nimport java.io.File\nimport android.view.View\n\nclass Bar\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:import-ordering");
    }

    // Oracle-verified: ktlint_official groups kotlinx before java/javax,
    // android_studio and `*` layout sort purely lexicographically.
    #[test]
    fn official_puts_kotlinx_before_javax() {
        assert!(official(
            "package foo\n\nimport kotlinx.coroutines.CoroutineScope\nimport javax.inject.Qualifier\n"
        )
        .is_empty());
        assert!(!official(
            "package foo\n\nimport javax.inject.Qualifier\nimport kotlinx.coroutines.CoroutineScope\n"
        )
        .is_empty());
    }

    #[test]
    fn android_studio_sorts_lexicographically() {
        assert!(android(
            "package foo\n\nimport javax.inject.Qualifier\nimport kotlinx.coroutines.CoroutineScope\n"
        )
        .is_empty());
        assert!(!android(
            "package foo\n\nimport kotlinx.coroutines.CoroutineScope\nimport javax.inject.Qualifier\n"
        )
        .is_empty());
    }

    #[test]
    fn star_layout_sorts_lexicographically() {
        assert!(star(
            "package foo\n\nimport javax.inject.Qualifier\nimport kotlinx.coroutines.CoroutineScope\n"
        )
        .is_empty());
        assert!(!star(
            "package foo\n\nimport kotlinx.coroutines.CoroutineScope\nimport javax.inject.Qualifier\n"
        )
        .is_empty());
    }

    #[test]
    fn case_sensitive_ascii() {
        // Z (0x5A) < a (0x61) — case-sensitive ASCII, verified vs ktlint.
        assert!(official("package foo\n\nimport Zeta.Foo\nimport alpha.Bar\n").is_empty());
        assert!(!official("package foo\n\nimport alpha.Bar\nimport Zeta.Foo\n").is_empty());
    }
}
