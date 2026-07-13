//! detekt style rules — code style checks. L0, text/CST level.
use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

pub struct NoTabs;
impl Rule for NoTabs {
    fn id(&self) -> &'static str { "detekt:style:NoTabs" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i, line)| {
            line.find('\t').map(|col| Violation {
                file: String::new(), line: i + 1, col: col + 1,
                rule_id: "detekt:style:NoTabs".into(),
                message: "Tab character found — use spaces".into(),
                auto_fixable: true,
            })
        }).collect()
    }
}

pub struct ForbiddenComment;
impl Rule for ForbiddenComment {
    fn id(&self) -> &'static str { "detekt:style:ForbiddenComment" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let keywords = ["TODO", "FIXME", "HACK", "XXX"];
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("//") || t.starts_with('*') || t.starts_with("/*") {
                for kw in &keywords {
                    if t.contains(kw) {
                        v.push(Violation {
                            file: String::new(), line: i + 1, col: t.find(kw).unwrap_or(0) + 1,
                            rule_id: "detekt:style:ForbiddenComment".into(),
                            message: format!("Forbidden comment marker: {}", kw),
                            auto_fixable: false,
                        });
                    }
                }
            }
        }
        v
    }
}

pub struct WildcardImport;
impl Rule for WildcardImport {
    fn id(&self) -> &'static str { "detekt:style:WildcardImport" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i, line)| {
            let t = line.trim();
            if t.starts_with("import ") && t.contains('*') {
                Some(Violation {
                    file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:style:WildcardImport".into(),
                    message: "Wildcard import should be avoided".into(),
                    auto_fixable: false,
                })
            } else { None }
        }).collect()
    }
}

pub struct MandatoryBracesIfElse;
impl Rule for MandatoryBracesIfElse {
    fn id(&self) -> &'static str { "detekt:style:MandatoryBracesIfElse" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_if_braces(tree.root_node(), source.as_bytes(), &mut v);
        v
    }
}

fn walk_if_braces(n: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
    if n.kind() == "if_expression" {
        let mut has_body = false;
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i) {
                if c.kind() == "control_structure_body" { has_body = true; break; }
            }
        }
        if !has_body {
            let pos = n.start_position();
            v.push(Violation {
                file: String::new(), line: pos.row + 1, col: pos.column + 1,
                rule_id: "detekt:style:MandatoryBracesIfElse".into(),
                message: "If/else branches should use braces".into(),
                auto_fixable: false,
            });
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { walk_if_braces(c, bytes, v); }
    }
}

// ── SpacingBetweenPackageAndImports ──
pub struct SpacingBetweenPackageAndImports;
impl Rule for SpacingBetweenPackageAndImports {
    fn id(&self) -> &'static str { "detekt:style:SpacingBetweenPackageAndImports" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _t: &Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let lines: Vec<&str> = s.lines().collect();
        let mut saw_package = false;
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("package ") { saw_package = true; continue; }
            if saw_package && !t.is_empty() && !t.starts_with("import ") && i>0 && !lines[i-1].trim().is_empty() {
                v.push(Violation {
                    file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:style:SpacingBetweenPackageAndImports".into(),
                    message: "Expected blank line between package and imports".into(),
                    auto_fixable: false,
                });
                saw_package = false;
            }
            if t.starts_with("import ") { saw_package = false; }
        }
        v
    }
}

// ── UseArrayLiteralsInAnnotations ──
pub struct UseArrayLiteralsInAnnotations;
impl Rule for UseArrayLiteralsInAnnotations {
    fn id(&self) -> &'static str { "detekt:style:UseArrayLiteralsInAnnotations" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _t: &Tree, s: &str) -> Vec<Violation> {
        s.lines().enumerate().filter_map(|(i, l)| {
            let t = l.trim();
            if t.starts_with('@') && t.contains("[") && t.contains(']') {
                Some(Violation {
                    file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:style:UseArrayLiteralsInAnnotations".into(),
                    message: "Use array literal syntax in annotations".into(),
                    auto_fixable: false,
                })
            } else { None }
        }).collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(r: &dyn Rule, s: &str) -> Vec<Violation> { r.check(&KotlinParser::new().parse(s), s) }

    #[test] fn no_tabs_ok() { assert!(c(&NoTabs, "fun f()\n").is_empty()); }
    #[test] fn no_tabs_bad() { assert!(!c(&NoTabs, "\tval x = 1\n").is_empty()); }
    #[test] fn forbidden_ok() { assert!(c(&ForbiddenComment, "// normal\n").is_empty()); }
    #[test] fn forbidden_bad() { assert!(!c(&ForbiddenComment, "// TODO\n").is_empty()); }
    #[test] fn wildcard_ok() { assert!(c(&WildcardImport, "import com.Foo\n").is_empty()); }
    #[test] fn wildcard_bad() { assert!(!c(&WildcardImport, "import com.*\n").is_empty()); }
}
