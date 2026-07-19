//! Type resolution bridge — provides type information to L2 rules.
//! Currently returns a best-effort type map extracted from CST annotations.
//! Future: optional FFI bridge to kotlinc for full type resolution.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DeclType {
    pub type_name: String,
    pub is_nullable: bool,
    pub line: usize,
}

#[derive(Debug, Clone, Default)]
pub struct TypeInfo {
    pub declarations: HashMap<String, DeclType>,
    pub return_types: HashMap<String, String>,
}

impl TypeInfo {
    pub fn extract(source: &str) -> Self {
        let mut ti = TypeInfo::default();
        let mut prev_line = "";
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("val ") || t.starts_with("var ") {
                if let Some((name, type_name, is_nullable)) = parse_property(t) {
                    ti.declarations.insert(
                        name,
                        DeclType {
                            type_name,
                            is_nullable,
                            line: i + 1,
                        },
                    );
                }
            }
            // Constructor parameters: class Foo(val x: Int, val y: String)
            if t.starts_with("class ") && !t.contains('{') {
                let class_line = format!("{} {}", prev_line, t);
                for (name, type_name, is_nullable) in extract_params(class_line.trim()) {
                    ti.declarations.insert(
                        name,
                        DeclType {
                            type_name,
                            is_nullable,
                            line: i + 1,
                        },
                    );
                }
            }
            if t.starts_with("fun ") {
                if let Some((name, ret_type)) = parse_function_return(t) {
                    ti.return_types.insert(name, ret_type);
                }
            }
            if t.contains('(') && (t.starts_with("fun ") || t.starts_with("class ")) {
                for (name, type_name, is_nullable) in extract_params(t) {
                    ti.declarations.insert(
                        name,
                        DeclType {
                            type_name,
                            is_nullable,
                            line: i + 1,
                        },
                    );
                }
            }
        }
        ti
    }

    pub fn type_of(&self, name: &str) -> Option<&DeclType> {
        self.declarations.get(name)
    }
}

fn parse_property(line: &str) -> Option<(String, String, bool)> {
    let rest = line.trim_start_matches("val ").trim_start_matches("var ");
    let parts: Vec<&str> = rest.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }
    let name = parts[0].trim().to_string();
    if name.contains(' ') || name.is_empty() {
        return None;
    }
    let type_part = parts[1].split('=').next().unwrap_or(parts[1]).trim();
    let is_nullable = type_part.ends_with('?');
    let type_name = type_part.trim_end_matches('?').trim().to_string();
    Some((name, type_name, is_nullable))
}

fn parse_function_return(line: &str) -> Option<(String, String)> {
    let rest = line.trim_start_matches("fun ");
    let name_end = rest.find('(')?;
    let name = rest[..name_end].trim().to_string();
    let after_open = &rest[name_end + 1..];
    let close = after_open.find(')')?;
    let after = &after_open[close + 1..];
    let ret = after.trim().trim_start_matches(':').trim();
    let ret = ret
        .split(|c: char| c == ' ' || c == '{')
        .next()
        .unwrap_or(ret)
        .trim();
    if ret.is_empty() {
        return None;
    }
    Some((name, ret.to_string()))
}

fn extract_params(line: &str) -> Vec<(String, String, bool)> {
    let mut params = Vec::new();
    let start = match line.find('(') {
        Some(p) => p,
        None => return params,
    };
    let end = match line.rfind(')') {
        Some(p) => p,
        None => return params,
    };
    if end <= start + 1 {
        return params;
    }
    for param in line[start + 1..end].split(',') {
        if let Some((name, type_name, is_nullable)) =
            parse_property(&format!("val {}", param.trim()))
        {
            params.push((name, type_name, is_nullable));
        }
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_property_non_null() {
        let ti = TypeInfo::extract("val x: String = \"hello\"");
        assert_eq!(ti.type_of("x").unwrap().type_name, "String");
        assert!(!ti.type_of("x").unwrap().is_nullable);
    }
    #[test]
    fn parse_property_nullable() {
        let ti = TypeInfo::extract("val y: String? = null");
        assert!(ti.type_of("y").unwrap().is_nullable);
    }
    #[test]
    fn parse_function_return_type() {
        let ti = TypeInfo::extract("fun foo(): Int { return 42 }");
        assert_eq!(ti.return_types.get("foo").unwrap(), "Int");
    }
    #[test]
    fn parse_constructor_params() {
        let ti = TypeInfo::extract("class Foo(val x: Int, val y: String?)");
        assert_eq!(ti.type_of("x").unwrap().type_name, "Int");
        assert!(ti.type_of("y").unwrap().is_nullable);
    }
    #[test]
    fn parse_function_params() {
        let ti = TypeInfo::extract("fun bar(x: Int, y: String?) {}");
        assert_eq!(ti.type_of("x").unwrap().type_name, "Int");
        assert!(ti.type_of("y").unwrap().is_nullable);
    }
}
