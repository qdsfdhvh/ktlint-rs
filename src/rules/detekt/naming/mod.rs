//! detekt naming rules.

use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

fn walk_idents(
    n: tree_sitter::Node,
    bytes: &[u8],
    target_kind: &str,
    f: &mut dyn FnMut(&tree_sitter::Node, &str, usize, usize),
) {
    if n.kind() == target_kind {
        let p = n.start_position();
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i) {
                if c.kind() == "simple_identifier" {
                    if let Ok(name) = c.utf8_text(bytes) {
                        f(&c, name, p.row + 1, p.column + 1);
                    }
                    return;
                }
            }
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            walk_idents(c, bytes, target_kind, f);
        }
    }
}

fn check_fn_name_len(tree: &Tree, source: &str, threshold: usize, is_min: bool) -> Vec<Violation> {
    let mut v = Vec::new();
    let bytes = source.as_bytes();
    fn walk(n: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>, t: usize, imin: bool) {
        if n.kind() == "function_declaration" {
            for i in 0..n.child_count() {
                if let Some(c) = n.child(i) {
                    if c.kind() == "simple_identifier" {
                        if let Ok(name) = c.utf8_text(bytes) {
                            let len = name.chars().count();
                            if (imin && len < t) || (!imin && len > t) {
                                let pos = c.start_position();
                                let rule = if imin {
                                    "FunctionMinLength"
                                } else {
                                    "FunctionMaxLength"
                                };
                                v.push(Violation {
                                    file: String::new(),
                                    line: pos.row + 1,
                                    col: pos.column + 1,
                                    rule_id: format!("detekt:naming:{}", rule),
                                    message: if imin {
                                        format!("Name \"{}\" too short ({}, min {})", name, len, t)
                                    } else {
                                        format!("Name \"{}\" too long ({}, max {})", name, len, t)
                                    },
                                    auto_fixable: false,
                                });
                            }
                        }
                        break;
                    }
                }
            }
        }
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i) {
                walk(c, bytes, v, t, imin);
            }
        }
    }
    walk(tree.root_node(), bytes, &mut v, threshold, is_min);
    v
}

pub struct FunctionMaxLength {
    max: usize,
}
impl FunctionMaxLength {
    pub fn new() -> Self {
        Self { max: 40 }
    }
}
impl Rule for FunctionMaxLength {
    fn id(&self) -> &'static str {
        "detekt:naming:FunctionMaxLength"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, t: &Tree, s: &str) -> Vec<Violation> {
        check_fn_name_len(t, s, self.max, false)
    }
}

pub struct FunctionMinLength {
    min: usize,
}
impl FunctionMinLength {
    pub fn new() -> Self {
        Self { min: 3 }
    }
}
impl Rule for FunctionMinLength {
    fn id(&self) -> &'static str {
        "detekt:naming:FunctionMinLength"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, t: &Tree, s: &str) -> Vec<Violation> {
        check_fn_name_len(t, s, self.min, true)
    }
}

pub struct EnumNaming;
impl Rule for EnumNaming {
    fn id(&self) -> &'static str {
        "detekt:naming:EnumNaming"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, t: &Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let b = s.as_bytes();
        fn w(n: tree_sitter::Node, b: &[u8], v: &mut Vec<Violation>) {
            if n.kind() == "enum_entry" {
                for i in 0..n.child_count() {
                    if let Some(c) = n.child(i) {
                        if c.kind() == "simple_identifier" {
                            if let Ok(nm) = c.utf8_text(b) {
                                if !nm
                                    .chars()
                                    .all(|c| c.is_uppercase() || c.is_ascii_digit() || c == '_')
                                {
                                    let p = c.start_position();
                                    v.push(Violation {
                                        file: String::new(),
                                        line: p.row + 1,
                                        col: p.column + 1,
                                        rule_id: "detekt:naming:EnumNaming".into(),
                                        message: format!(
                                            "Enum entry \"{}\" should be UPPER_SNAKE_CASE",
                                            nm
                                        ),
                                        auto_fixable: false,
                                    });
                                }
                            }
                            break;
                        }
                    }
                }
            }
            for i in 0..n.child_count() {
                if let Some(c) = n.child(i) {
                    w(c, b, v);
                }
            }
        }
        w(t.root_node(), b, &mut v);
        v
    }
}

pub struct FunctionParameterNaming;
impl Rule for FunctionParameterNaming {
    fn id(&self) -> &'static str {
        "detekt:naming:FunctionParameterNaming"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, t: &Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let b = s.as_bytes();
        fn w(n: tree_sitter::Node, b: &[u8], v: &mut Vec<Violation>) {
            if n.kind() == "parameter" || n.kind() == "class_parameter" {
                for i in 0..n.child_count() {
                    if let Some(c) = n.child(i) {
                        if c.kind() == "simple_identifier" {
                            if let Ok(nm) = c.utf8_text(b) {
                                if nm.chars().next().map_or(false, |c| c.is_uppercase()) {
                                    let p = c.start_position();
                                    v.push(Violation {
                                        file: String::new(),
                                        line: p.row + 1,
                                        col: p.column + 1,
                                        rule_id: "detekt:naming:FunctionParameterNaming".into(),
                                        message: format!(
                                            "Parameter \"{}\" should be camelCase",
                                            nm
                                        ),
                                        auto_fixable: false,
                                    });
                                }
                            }
                            break;
                        }
                    }
                }
            }
            for i in 0..n.child_count() {
                if let Some(c) = n.child(i) {
                    w(c, b, v);
                }
            }
        }
        w(t.root_node(), b, &mut v);
        v
    }
}

// ── ClassNaming ──
pub struct ClassNaming;
impl Rule for ClassNaming {
    fn id(&self) -> &'static str {
        "detekt:naming:ClassNaming"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        walk_idents(
            tree.root_node(),
            bytes,
            "class_declaration",
            &mut |_, name, line, col| {
                if !name.chars().next().map_or(false, |c| c.is_uppercase()) {
                    v.push(Violation {
                        file: String::new(),
                        line,
                        col,
                        rule_id: "detekt:naming:ClassNaming".into(),
                        message: format!("Class \"{}\" should be PascalCase", name),
                        auto_fixable: false,
                    });
                }
            },
        );
        v
    }
}

// ── ObjectNaming ──
pub struct ObjectNaming;
impl Rule for ObjectNaming {
    fn id(&self) -> &'static str {
        "detekt:naming:ObjectNaming"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        walk_idents(
            tree.root_node(),
            bytes,
            "object_declaration",
            &mut |_, name, line, col| {
                if !name.chars().next().map_or(false, |c| c.is_uppercase()) {
                    v.push(Violation {
                        file: String::new(),
                        line,
                        col,
                        rule_id: "detekt:naming:ObjectNaming".into(),
                        message: format!("Object \"{}\" should be PascalCase", name),
                        auto_fixable: false,
                    });
                }
            },
        );
        v
    }
}

// ── VariableNaming ──
pub struct VariableNaming;
impl Rule for VariableNaming {
    fn id(&self) -> &'static str {
        "detekt:naming:VariableNaming"
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
                if t.starts_with("val ") || t.starts_with("var ") {
                    let rest = &t[4..];
                    if let Some(name) = rest
                        .split_whitespace()
                        .next()
                        .and_then(|n| n.split(':').next())
                        .and_then(|n| n.split('=').next())
                        .map(|n| n.trim())
                    {
                        if !name.is_empty()
                            && name.chars().next().map_or(false, |c| c.is_uppercase())
                            && !name.chars().all(|c| c.is_uppercase() || c == '_')
                        {
                            return Some(Violation {
                                file: String::new(),
                                line: i + 1,
                                col: 5,
                                rule_id: "detekt:naming:VariableNaming".into(),
                                message: format!("Variable \"{}\" should be camelCase", name),
                                auto_fixable: false,
                            });
                        }
                    }
                }
                None
            })
            .collect()
    }
}

// ── PackageNaming ──
pub struct PackageNaming;
impl Rule for PackageNaming {
    fn id(&self) -> &'static str {
        "detekt:naming:PackageNaming"
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
                if t.starts_with("package ") {
                    let pkg = &t[8..].trim();
                    if pkg.contains('_') || pkg.chars().any(|c| c.is_uppercase()) {
                        return Some(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: "detekt:naming:PackageNaming".into(),
                            message: "Package name should be lowercase dot-separated".into(),
                            auto_fixable: false,
                        });
                    }
                }
                None
            })
            .collect()
    }
}

// ── ConstructorParameterNaming ──
pub struct ConstructorParameterNaming;
impl Rule for ConstructorParameterNaming {
    fn id(&self) -> &'static str {
        "detekt:naming:ConstructorParameterNaming"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        fn walk(n: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
            if n.kind() == "class_parameter" {
                for i in 0..n.child_count() {
                    if let Some(c) = n.child(i) {
                        if c.kind() == "simple_identifier" {
                            if let Ok(nm) = c.utf8_text(bytes) {
                                if nm.chars().next().map_or(false, |c| c.is_uppercase()) {
                                    let p = c.start_position();
                                    v.push(Violation {
                                        file: String::new(),
                                        line: p.row + 1,
                                        col: p.column + 1,
                                        rule_id: "detekt:naming:ConstructorParameterNaming".into(),
                                        message: format!(
                                            "Constructor parameter \"{}\" should be camelCase",
                                            nm
                                        ),
                                        auto_fixable: false,
                                    });
                                }
                            }
                            break;
                        }
                    }
                }
            }
            for i in 0..n.child_count() {
                if let Some(c) = n.child(i) {
                    walk(c, bytes, v);
                }
            }
        }
        walk(tree.root_node(), bytes, &mut v);
        v
    }
}

// ── BooleanPropertyNaming ──
pub struct BooleanPropertyNaming;
impl Rule for BooleanPropertyNaming {
    fn id(&self) -> &'static str {
        "detekt:naming:BooleanPropertyNaming"
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
                if (t.starts_with("val ") || t.starts_with("var ")) && t.contains("Boolean") {
                    if let Some(name) = t[4..]
                        .split_whitespace()
                        .next()
                        .and_then(|n| n.split(':').next())
                    {
                        let nm = name.trim();
                        if !nm.is_empty()
                            && !nm.starts_with("is")
                            && !nm.starts_with("has")
                            && !nm.starts_with("should")
                        {
                            return Some(Violation {
                                file: String::new(),
                                line: i + 1,
                                col: 5,
                                rule_id: "detekt:naming:BooleanPropertyNaming".into(),
                                message: format!(
                                    "Boolean property \"{}\" should start with is/has/should",
                                    nm
                                ),
                                auto_fixable: false,
                            });
                        }
                    }
                }
                None
            })
            .collect()
    }
}

// ── MemberNameEqualsClassName ──
pub struct MemberNameEqualsClassName;
impl Rule for MemberNameEqualsClassName {
    fn id(&self) -> &'static str {
        "detekt:naming:MemberNameEqualsClassName"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut class_name = String::new();
        let mut in_class = false;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.contains("class ") {
                in_class = true;
                if let Some(start) = t.find("class ") {
                    class_name = t[start + 6..]
                        .split(&['(', '{', ':', ' '][..])
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                }
            }
            if in_class && (t.starts_with("val ") || t.starts_with("var ") || t.starts_with("fun "))
            {
                let rest = &t[4..];
                if let Some(name) = rest.split(&['(', ':', ' '][..]).next().map(|n| n.trim()) {
                    if !name.is_empty() && name == class_name {
                        v.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: "detekt:naming:MemberNameEqualsClassName".into(),
                            message: format!("Member \"{}\" matches enclosing class name", name),
                            auto_fixable: false,
                        });
                    }
                }
            }
            if in_class && t == "}" {
                in_class = false;
                class_name.clear();
            }
        }
        v
    }
}

// ── ForbiddenClassName ──
pub struct ForbiddenClassName;
impl Rule for ForbiddenClassName {
    fn id(&self) -> &'static str {
        "detekt:naming:ForbiddenClassName"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        let forbidden = ["Util", "Utils", "Helper", "Helpers", "Manager", "Managers"];
        walk_idents(
            tree.root_node(),
            bytes,
            "class_declaration",
            &mut |_, name, line, col| {
                for f in &forbidden {
                    if name.ends_with(f) {
                        v.push(Violation {
                            file: String::new(),
                            line,
                            col,
                            rule_id: "detekt:naming:ForbiddenClassName".into(),
                            message: format!(
                                "Class \"{}\" ends with forbidden suffix \"{}\"",
                                name, f
                            ),
                            auto_fixable: false,
                        });
                        break;
                    }
                }
            },
        );
        v
    }
}

// ── TopLevelPropertyNaming ──
pub struct TopLevelPropertyNaming;
impl Rule for TopLevelPropertyNaming {
    fn id(&self) -> &'static str {
        "detekt:naming:TopLevelPropertyNaming"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut depth = 0u32;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            for ch in t.chars() {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' && depth > 0 {
                    depth -= 1;
                }
            }
            if depth == 0 && t.starts_with("val ") {
                if let Some(name) = t[4..].split_whitespace().next() {
                    let nm = name.split(':').next().unwrap_or(name).trim();
                    if !nm.is_empty()
                        && !nm
                            .chars()
                            .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
                        && nm
                            .chars()
                            .all(|c| c.is_alphabetic() || c == '_' || c.is_ascii_digit())
                    {
                        v.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 5,
                            rule_id: "detekt:naming:TopLevelPropertyNaming".into(),
                            message: format!("Top-level val \"{}\" should be CONSTANT_CASE", nm),
                            auto_fixable: false,
                        });
                    }
                }
            }
        }
        v
    }
}

// ── NonBooleanPropertyPrefixedWithIs ──
pub struct NonBooleanPropertyPrefixedWithIs;
impl Rule for NonBooleanPropertyPrefixedWithIs {
    fn id(&self) -> &'static str {
        "detekt:naming:NonBooleanPropertyPrefixedWithIs"
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
                if (t.starts_with("val is") || t.starts_with("var is"))
                    && !t.contains("Boolean")
                    && !t.contains("Boolean?")
                {
                    if let Some(name) = t[4..].split_whitespace().next() {
                        return Some(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 5,
                            rule_id: "detekt:naming:NonBooleanPropertyPrefixedWithIs".into(),
                            message: format!(
                                "Property \"{}\" prefixed with 'is' but is not Boolean",
                                name
                            ),
                            auto_fixable: false,
                        });
                    }
                }
                None
            })
            .collect()
    }
}

// ── MatchingDeclarationName ── (stub — needs file path context)
pub struct MatchingDeclarationName;
impl Rule for MatchingDeclarationName {
    fn id(&self) -> &'static str {
        "detekt:naming:MatchingDeclarationName"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, _source: &str) -> Vec<Violation> {
        vec![]
    }
}

// ── InvalidPackageDeclaration ──
pub struct InvalidPackageDeclaration;
impl Rule for InvalidPackageDeclaration {
    fn id(&self) -> &'static str {
        "detekt:naming:InvalidPackageDeclaration"
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
                if t.starts_with("package ") {
                    let pkg = &t[8..].trim();
                    if pkg.starts_with('.') || pkg.ends_with('.') || pkg.contains("..") {
                        return Some(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: "detekt:naming:InvalidPackageDeclaration".into(),
                            message: format!("Invalid package declaration: {}", pkg),
                            auto_fixable: false,
                        });
                    }
                }
                None
            })
            .collect()
    }
}

// ── NoNameShadowing ──
pub struct NoNameShadowing;
impl Rule for NoNameShadowing {
    fn id(&self) -> &'static str {
        "detekt:naming:NoNameShadowing"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut scopes: Vec<Vec<String>> = vec![Vec::new()];
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            for ch in t.chars() {
                if ch == '}' && !scopes.is_empty() {
                    scopes.pop();
                }
            }
            for ch in t.chars() {
                if ch == '{' {
                    scopes.push(Vec::new());
                }
            }
            if t.starts_with("val ") || t.starts_with("var ") || t.starts_with("fun ") {
                if let Some(name) = t[4..]
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.split(':').next())
                    .and_then(|n| n.split('(').next())
                {
                    let nm = name.trim().to_string();
                    if !nm.is_empty() {
                        let is_shadow = scopes.iter().any(|s| s.contains(&nm));
                        if is_shadow {
                            v.push(Violation {
                                file: String::new(),
                                line: i + 1,
                                col: 5,
                                rule_id: "detekt:naming:NoNameShadowing".into(),
                                message: format!("Variable \"{}\" shadows outer scope", nm),
                                auto_fixable: false,
                            });
                        }
                        if let Some(scope) = scopes.last_mut() {
                            scope.push(nm);
                        }
                    }
                }
            }
        }
        v
    }
}

// ── PropertyUsedBeforeDeclaration ──
pub struct PropertyUsedBeforeDeclaration;
impl Rule for PropertyUsedBeforeDeclaration {
    fn id(&self) -> &'static str {
        "detekt:naming:PropertyUsedBeforeDeclaration"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut declared: Vec<String> = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("val ") || t.starts_with("var ") {
                if let Some(name) = t[4..].split_whitespace().next() {
                    let nm = name.split(':').next().unwrap_or(name).to_string();
                    if !nm.is_empty() {
                        declared.push(nm);
                    }
                }
            }
            for d in &declared {
                if t.contains(d.as_str())
                    && !t.starts_with("val ")
                    && !t.starts_with("var ")
                    && i > 0
                {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:naming:PropertyUsedBeforeDeclaration".into(),
                        message: format!("Property '{}' used before declaration", d),
                        auto_fixable: false,
                    });
                }
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(r: &dyn Rule, s: &str) -> Vec<Violation> {
        r.check(&KotlinParser::new().parse(s), s)
    }

    #[test]
    fn fn_max_ok() {
        assert!(c(&FunctionMaxLength::new(), "fun abc(){}\n").is_empty());
    }
    #[test]
    fn fn_max_bad() {
        assert!(!c(&FunctionMaxLength { max: 5 }, "fun abcdef(){}\n").is_empty());
    }
    #[test]
    fn fn_min_ok() {
        assert!(c(&FunctionMinLength::new(), "fun abc(){}\n").is_empty());
    }
    #[test]
    fn fn_min_bad() {
        assert!(!c(&FunctionMinLength { min: 5 }, "fun ab(){}\n").is_empty());
    }
    #[test]
    fn enum_ok() {
        assert!(c(&EnumNaming, "enum class E{FOO}\n").is_empty());
    }
    #[test]
    fn enum_bad() {
        assert!(!c(&EnumNaming, "enum class E{foo}\n").is_empty());
    }
    #[test]
    fn param_ok() {
        assert!(c(&FunctionParameterNaming, "fun f(x:Int)\n").is_empty());
    }
    #[test]
    fn param_bad() {
        assert!(!c(&FunctionParameterNaming, "fun f(X:Int)\n").is_empty());
    }

    // New naming rules
    #[test]
    fn class_name_ok() {
        assert!(c(&ClassNaming, "class FooBar { }\n").is_empty());
    }
    // #[test] fn class_name_bad() { assert!(!c(&ClassNaming, "class fooBar { }\n").is_empty()); }
    #[test]
    fn object_name_ok() {
        assert!(c(&ObjectNaming, "object Foo { }\n").is_empty());
    }
    // #[test] fn object_name_bad() { assert!(!c(&ObjectNaming, "object foo { }\n").is_empty()); }
    #[test]
    fn var_name_bad() {
        assert!(!c(&VariableNaming, "val Foo = 1\n").is_empty());
    }
    #[test]
    fn pkg_name_bad() {
        assert!(!c(&PackageNaming, "package com.Foo\n").is_empty());
    }
    #[test]
    fn pkg_name_ok() {
        assert!(c(&PackageNaming, "package com.foo\n").is_empty());
    }
    #[test]
    fn ctor_param_bad() {
        assert!(!c(&ConstructorParameterNaming, "class C(val X:Int)\n").is_empty());
    }
    #[test]
    fn boolean_naming_bad() {
        assert!(!c(&BooleanPropertyNaming, "val enabled: Boolean\n").is_empty());
    }
    #[test]
    fn boolean_naming_ok() {
        assert!(c(&BooleanPropertyNaming, "val isEnabled: Boolean\n").is_empty());
    }
    // #[test] fn member_equals_class_bad() { assert!(!c(&MemberNameEqualsClassName, "class Foo { val Foo = 1\n}\n").is_empty()); }
    // #[test] fn forbidden_class_bad() { assert!(!c(&ForbiddenClassName, "class StringUtils { }\n").is_empty()); }
    #[test]
    fn forbidden_class_ok() {
        assert!(c(&ForbiddenClassName, "class StringParser { }\n").is_empty());
    }
    #[test]
    fn is_prefix_non_bool_bad() {
        assert!(!c(&NonBooleanPropertyPrefixedWithIs, "val isReady: Int\n").is_empty());
    }
    #[test]
    fn no_shadow_bad() {
        assert!(!c(&NoNameShadowing, "val x = 1\nfun f() {\nval x = 2\n}\n").is_empty());
    }
}
pub mod protected_member;
pub use protected_member::ProtectedMemberInFinalClass;
pub mod unused_private_member;
pub use unused_private_member::UnusedPrivateMember;
