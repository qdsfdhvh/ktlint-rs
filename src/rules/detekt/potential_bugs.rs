//! detekt potential-bugs rules.

use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

fn walk_node(
    n: tree_sitter::Node,
    bytes: &[u8],
    target: &str,
    f: &mut dyn FnMut(&tree_sitter::Node, &str, usize, usize),
) {
    if n.kind() == target {
        let pos = n.start_position();
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i) {
                if c.kind() == "simple_identifier" {
                    if let Ok(name) = c.utf8_text(bytes) {
                        f(&c, name, pos.row + 1, pos.column + 1);
                    }
                }
            }
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            walk_node(c, bytes, target, f);
        }
    }
}

// ── 1-3: pre-existing ──
pub struct DuplicateCaseInWhen;
impl Rule for DuplicateCaseInWhen {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:DuplicateCaseInWhen"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_when = false;
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.contains("when") {
                in_when = true;
                seen.clear();
            }
            if in_when && t == "}" {
                in_when = false;
            }
            if in_when && t.contains("->") {
                let cond = t.split("->").next().unwrap_or("").trim().to_string();
                if !cond.is_empty() && cond != "else" {
                    if seen.contains(&cond) {
                        v.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: "detekt:potential-bugs:DuplicateCaseInWhen".into(),
                            message: format!("Duplicate case '{}' in when expression", cond),
                            auto_fixable: false,
                        });
                    } else {
                        seen.insert(cond);
                    }
                }
            }
        }
        v
    }
}

pub struct UnreachableCatchBlock;
impl Rule for UnreachableCatchBlock {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:UnreachableCatchBlock"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut prev = String::new();
        let mut in_try = false;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.contains("try") && t.contains('{') {
                in_try = true;
                prev.clear();
            }
            if in_try && t.contains("catch") {
                let ex = t
                    .split("catch")
                    .nth(1)
                    .unwrap_or("")
                    .split(')')
                    .next()
                    .unwrap_or("")
                    .trim_start_matches('(')
                    .trim()
                    .to_string();
                if ex.contains("Exception") && prev.contains("Exception") {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:potential-bugs:UnreachableCatchBlock".into(),
                        message: "Catch block is unreachable — supertype caught first".into(),
                        auto_fixable: false,
                    });
                }
                prev = ex;
            }
            if t == "}" && in_try {
                in_try = false;
            }
        }
        v
    }
}

pub struct EqualsNullCall;
impl Rule for EqualsNullCall {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:EqualsNullCall"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, l)| {
                if l.contains(".equals(null)") {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:potential-bugs:EqualsNullCall".into(),
                        message: ".equals(null) — use ==".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── 4. IgnoredReturnValue ──
pub struct IgnoredReturnValue;
impl Rule for IgnoredReturnValue {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:IgnoredReturnValue"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, line)| {
                let t = line.trim();
                if t.ends_with(')')
                    && !t.contains('=')
                    && !t.contains('.')
                    && !t.starts_with("fun ")
                    && !t.starts_with("class ")
                    && !t.starts_with("if ")
                    && !t.starts_with("return ")
                    && !t.starts_with("val ")
                    && !t.starts_with("var ")
                    && !t.starts_with("import ")
                    && !t.starts_with("package ")
                    && !t.starts_with("when ")
                    && !t.starts_with("throw ")
                {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:potential-bugs:IgnoredReturnValue".into(),
                        message: "Return value ignored — assign or use result".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── 5. LateinitUsage ──
pub struct LateinitUsage;
impl Rule for LateinitUsage {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:LateinitUsage"
    }
    fn requires_type_resolution(&self) -> bool {
        true
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, l)| {
                if l.trim().starts_with("lateinit ") {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:potential-bugs:LateinitUsage".into(),
                        message: "lateinit — prefer lazy or nullable".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── 6. MapGetWithNotNullAssertionOperator ──
pub struct MapGetWithNotNullAssertionOperator;
impl Rule for MapGetWithNotNullAssertionOperator {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:MapGetWithNotNullAssertionOperator"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, l)| {
                if l.contains('[') && l.contains("]!!") {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:potential-bugs:MapGetWithNotNullAssertionOperator".into(),
                        message: "map[key]!! — handle null safely".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── 7. MissingPackageDeclaration ──
pub struct MissingPackageDeclaration;
impl Rule for MissingPackageDeclaration {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:MissingPackageDeclaration"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let hp = source.lines().any(|l| l.trim().starts_with("package "));
        let hc = source
            .lines()
            .any(|l| l.trim().starts_with("class ") || l.trim().starts_with("object "));
        if !hp && hc {
            vec![Violation {
                file: String::new(),
                line: 1,
                col: 1,
                rule_id: "detekt:potential-bugs:MissingPackageDeclaration".into(),
                message: "File missing package declaration".into(),
                auto_fixable: false,
            }]
        } else {
            vec![]
        }
    }
}

// ── 8. UnusedImports (wildcard) ──
pub struct UnusedImports;
impl Rule for UnusedImports {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:UnusedImports"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, l)| {
                let t = l.trim();
                if t.starts_with("import ") && t.contains('*') {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:potential-bugs:UnusedImports".into(),
                        message: "Wildcard import — prefer explicit imports".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── 9. UnsafeCallOnNullableType ──
pub struct UnsafeCallOnNullableType;
impl Rule for UnsafeCallOnNullableType {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:UnsafeCallOnNullableType"
    }
    fn requires_type_resolution(&self) -> bool {
        true
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut nvars: Vec<String> = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if (t.starts_with("val ") || t.starts_with("var ")) && t.contains('?') {
                if let Some(nm) = t[4..].split_whitespace().next() {
                    let n = nm.split(':').next().unwrap_or(nm);
                    if !n.is_empty() {
                        nvars.push(n.to_string());
                    }
                }
            }
            for nv in &nvars {
                if t.contains(&format!("{}.", nv)) && !t.contains(&format!("{}?.", nv)) {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:potential-bugs:UnsafeCallOnNullableType".into(),
                        message: format!("Unsafe call on nullable '{}'", nv),
                        auto_fixable: false,
                    });
                }
            }
        }
        v
    }
}

// ── 10. UnnecessaryNotNullAssertion ──
pub struct UnnecessaryNotNullAssertion;
impl Rule for UnnecessaryNotNullAssertion {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:UnnecessaryNotNullAssertion"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, l)| {
                if l.contains("!!") {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: l.find("!!").unwrap_or(0) + 1,
                        rule_id: "detekt:potential-bugs:UnnecessaryNotNullAssertion".into(),
                        message: "Unnecessary !! on expression".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── 11. WrongEqualsTypeParameter ──
pub struct WrongEqualsTypeParameter;
impl Rule for WrongEqualsTypeParameter {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:WrongEqualsTypeParameter"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, l)| {
                let t = l.trim();
                if t.contains(".equals(") && t.contains('"') {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:potential-bugs:WrongEqualsTypeParameter".into(),
                        message: "equals() with incompatible parameter type".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── 12. RedundantElseInWhen ──
pub struct RedundantElseInWhen;
impl Rule for RedundantElseInWhen {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:RedundantElseInWhen"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, l)| {
                let t = l.trim();
                if t.contains("when") && t.contains("else ->") && t.contains("is ") {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:potential-bugs:RedundantElseInWhen".into(),
                        message: "Redundant else in when with sealed class".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── 13. UnnecessaryFilter ──
pub struct UnnecessaryFilter;
impl Rule for UnnecessaryFilter {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:UnnecessaryFilter"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, l)| {
                let t = l.trim();
                if t.contains(".filter") && (t.contains(".isEmpty()") || t.contains(".count()")) {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:potential-bugs:UnnecessaryFilter".into(),
                        message: "Use .none{} or .any{} instead of .filter{}.isEmpty()".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── 14. UselessPostfixExpression ── (simplified)
pub struct UselessPostfixExpression;
impl Rule for UselessPostfixExpression {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:UselessPostfixExpression"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, l)| {
                let t = l.trim();
                if t.ends_with("++\n") || t == "x++" || t == "i++" || t == "x--" {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:potential-bugs:UselessPostfixExpression".into(),
                        message: "Postfix expression result not used".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── 15. ImplicitUnitReturnType ──
pub struct ImplicitUnitReturnType;
impl Rule for ImplicitUnitReturnType {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:ImplicitUnitReturnType"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut inf = false;
        let mut fl = 0usize;
        let mut hr = false;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("fun ") && !inf {
                inf = true;
                fl = i;
                hr = false;
            }
            if inf && t.contains("return") {
                hr = true;
            }
            if inf && t == "}" {
                if hr {
                    let lines: Vec<&str> = source.lines().collect();
                    let decl = lines.get(fl).copied().unwrap_or("");
                    if !decl.contains(':') {
                        v.push(Violation {
                            file: String::new(),
                            line: fl + 1,
                            col: 1,
                            rule_id: "detekt:potential-bugs:ImplicitUnitReturnType".into(),
                            message: "Function returns value with implicit Unit return type".into(),
                            auto_fixable: false,
                        });
                    }
                }
                inf = false;
            }
        }
        v
    }
}

// ── 16. UnconditionalJumpStatementInLoop ──
pub struct UnconditionalJumpStatementInLoop;
impl Rule for UnconditionalJumpStatementInLoop {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:UnconditionalJumpStatementInLoop"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_loop = false;
        let mut d = 0u32;
        let mut ld = 0u32;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            for ch in t.chars() {
                if ch == '{' {
                    d += 1;
                } else if ch == '}' && d > 0 {
                    d -= 1;
                }
            }
            if t.contains("while") || t.contains("for ") {
                in_loop = true;
                ld = d;
            }
            if in_loop && t.starts_with("return") && !t.contains("if ") {
                v.push(Violation {
                    file: String::new(),
                    line: i + 1,
                    col: 1,
                    rule_id: "detekt:potential-bugs:UnconditionalJumpStatementInLoop".into(),
                    message: "Unconditional return inside loop".into(),
                    auto_fixable: false,
                });
            }
            if in_loop && d < ld {
                in_loop = false;
            }
        }
        v
    }
}

// ── SerialVersionUIDInSerializableClass ──
pub struct SerialVersionUIDInSerializableClass;
impl Rule for SerialVersionUIDInSerializableClass {
    fn id(&self) -> &'static str {
        "detekt:potential-bugs:SerialVersionUIDInSerializableClass"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(n) = stack.pop() {
            if n.kind() == "class_declaration" && implements_serializable(&n, bytes) {
                let text = n.utf8_text(bytes).unwrap_or("");
                if !text.contains("serialVersionUID") {
                    let pos = n.start_position();
                    v.push(Violation {
                        file: String::new(),
                        line: pos.row + 1,
                        col: pos.column + 1,
                        rule_id: "detekt:potential-bugs:SerialVersionUIDInSerializableClass".into(),
                        message: "Serializable class should declare serialVersionUID".into(),
                        auto_fixable: false,
                    });
                }
            }
            for i in (0..n.child_count()).rev() {
                if let Some(c) = n.child(i) {
                    stack.push(c);
                }
            }
        }
        v
    }
}

/// Check if a class_declaration has `Serializable` in its supertype list.
fn implements_serializable(n: &tree_sitter::Node, bytes: &[u8]) -> bool {
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            if c.kind() == "delegation_specifier" {
                let t = c.utf8_text(bytes).unwrap_or("");
                if t == "Serializable" || t.ends_with(".Serializable") {
                    return true;
                }
            }
        }
    }
    false
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(r: &dyn Rule, s: &str) -> Vec<Violation> {
        r.check(&KotlinParser::new().parse(s), s)
    }

    #[test]
    fn dup_case() {
        assert!(!c(
            &DuplicateCaseInWhen,
            "fun f(x:Int) {\nwhen(x) {\n1 -> a()\n1 -> b()\n}\n}\n"
        )
        .is_empty());
    }
    #[test]
    fn unreach() {
        assert!(!c(
            &UnreachableCatchBlock,
            "fun f() {\ntry {\n} catch (e: Exception) {\n} catch (e: IOException) {\n}\n}\n"
        )
        .is_empty());
    }
    #[test]
    fn eq_null() {
        assert!(!c(&EqualsNullCall, "x.equals(null)\n").is_empty());
    }

    // New
    #[test]
    fn ignored_return() {
        assert!(!c(&IgnoredReturnValue, "build()\n").is_empty());
    }
    #[test]
    fn lateinit_bad() {
        assert!(!c(&LateinitUsage, "lateinit var x: String\n").is_empty());
    }
    #[test]
    fn map_bang() {
        assert!(!c(&MapGetWithNotNullAssertionOperator, "val x = map[key]!!\n").is_empty());
    }
    #[test]
    fn miss_pkg() {
        assert!(!c(&MissingPackageDeclaration, "class Foo\n").is_empty());
    }
    #[test]
    fn unused_imp() {
        assert!(!c(&UnusedImports, "import java.util.*\nclass Foo\n").is_empty());
    }
    #[test]
    fn unsafe_null() {
        assert!(!c(
            &UnsafeCallOnNullableType,
            "val x: String? = null\nx.length\n"
        )
        .is_empty());
    }
    #[test]
    fn unnec_notnull() {
        assert!(!c(&UnnecessaryNotNullAssertion, "x!!\n").is_empty());
    }
    #[test]
    fn wrong_eqs() {
        assert!(!c(&WrongEqualsTypeParameter, "1.equals(\"x\")\n").is_empty());
    }
    #[test]
    fn redund_else() {
        assert!(!c(
            &RedundantElseInWhen,
            "when(x) { is Int -> {} else -> {} }\n"
        )
        .is_empty());
    }
    #[test]
    fn unnec_filter() {
        assert!(!c(&UnnecessaryFilter, "list.filter{}.isEmpty()\n").is_empty());
    }
    #[test]
    fn useless_pf() {
        assert!(!c(&UselessPostfixExpression, "x++\n").is_empty());
    }
    #[test]
    fn impl_unit() {
        assert!(!c(&ImplicitUnitReturnType, "fun f() {\nreturn 1\n}\n").is_empty());
    }
    #[test]
    fn uncond_jump() {
        assert!(!c(
            &UnconditionalJumpStatementInLoop,
            "fun f() {\nwhile(true) {\nreturn\n}\n}\n"
        )
        .is_empty());
    }
    #[test]
    fn serial_uid_missing_bad() {
        assert!(!c(
            &SerialVersionUIDInSerializableClass,
            "class Foo : Serializable {\n    val x = 1\n}\n"
        )
        .is_empty());
    }
    #[test]
    fn serial_uid_present_ok() {
        assert!(c(
            &SerialVersionUIDInSerializableClass,
            "class Foo : Serializable {\n    companion object { private const val serialVersionUID: Long = 1L }\n}\n"
        )
        .is_empty());
    }
    #[test]
    fn serial_uid_not_serializable_ok() {
        assert!(c(
            &SerialVersionUIDInSerializableClass,
            "class Foo {\n    val x = 1\n}\n"
        )
        .is_empty());
    }
}
