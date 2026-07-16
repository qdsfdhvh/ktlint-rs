//! Builds a SymbolTable by walking the tree-sitter CST.

use super::{SymbolKind, SymbolTable, Visibility};
use tree_sitter::Node;

/// Build a SymbolTable from a tree-sitter CST.
pub fn build_symbol_table(source: &str, root: Node) -> SymbolTable {
    let mut table = SymbolTable::new();
    let bytes = source.as_bytes();
    walk_declarations(root, bytes, &mut table, 0);
    table
}

fn walk_declarations(node: Node, bytes: &[u8], table: &mut SymbolTable, scope_id: usize) {
    let mut stack: Vec<(Node, usize)> = vec![(node, scope_id)];

    while let Some((n, sid)) = stack.pop() {
        match n.kind() {
            "class_declaration" | "object_declaration" | "enum_class" => {
                if let Some(sym) = extract_class_symbol(&n, bytes) {
                    let new_scope = table.add_scope(sid);
                    table.add_symbol(
                        sym.name.clone(),
                        sym.kind,
                        sym.visibility,
                        sym.line,
                        sym.col,
                        sid,
                    );
                    // Push children into the new scope
                    push_children(&n, &mut stack, new_scope);
                }
            }

            "function_declaration" => {
                if let Some(sym) = extract_function_symbol(&n, bytes) {
                    let new_scope = table.add_scope(sid);
                    table.add_symbol(
                        sym.name.clone(),
                        SymbolKind::Function,
                        sym.visibility,
                        sym.line,
                        sym.col,
                        sid,
                    );
                    push_children(&n, &mut stack, new_scope);
                }
            }

            "property_declaration" => {
                if let Some(name) = extract_property_name(&n, bytes) {
                    let vis = extract_visibility(&n, bytes);
                    let pos = n.start_position();
                    table.add_symbol(
                        name,
                        SymbolKind::Property,
                        vis,
                        pos.row + 1,
                        pos.column + 1,
                        sid,
                    );
                }
                push_children(&n, &mut stack, sid);
            }

            "import_header" => {
                extract_imports(&n, bytes, table);
            }
            // ── Function body → new scope for locals ──
            "function_body" => {
                let inner = table.add_scope(sid);
                push_children(&n, &mut stack, inner);
            }
            // ── Function parameters ──
            "parameter" => {
                for ci in 0..n.child_count() {
                    if let Some(c) = n.child(ci) {
                        if c.kind() == "simple_identifier" || c.kind() == "identifier" {
                            let name = c.utf8_text(bytes).unwrap_or("").to_string();
                            if !name.is_empty() {
                                let pos = n.start_position();
                                table.add_symbol(
                                    name,
                                    SymbolKind::Property,
                                    Visibility::Implicit,
                                    pos.row + 1,
                                    pos.column + 1,
                                    sid,
                                );
                            }
                            break;
                        }
                    }
                }
                push_children(&n, &mut stack, sid);
            }

            // ── Constructors ──
            "secondary_constructor" | "primary_constructor" => {
                table.add_symbol(
                    "<init>".into(),
                    SymbolKind::Constructor,
                    Visibility::Implicit,
                    n.start_position().row + 1,
                    n.start_position().column + 1,
                    sid,
                );
                push_children(&n, &mut stack, sid);
            }

            // ── Function parameters ──
            "value_parameter" => {
                let pos = n.start_position();
                for ci in 0..n.child_count() {
                    if let Some(c) = n.child(ci) {
                        if c.kind() == "simple_identifier" || c.kind() == "identifier" {
                            let name = c.utf8_text(bytes).unwrap_or("").to_string();
                            if !name.is_empty() {
                                table.add_symbol(
                                    name,
                                    SymbolKind::Property,
                                    Visibility::Implicit,
                                    pos.row + 1,
                                    pos.column + 1,
                                    sid,
                                );
                            }
                            break;
                        }
                    }
                }
                push_children(&n, &mut stack, sid);
            }

            // ── Function parameters ──
            "value_parameter" => {
                let pos = n.start_position();
                for ci in 0..n.child_count() {
                    if let Some(c) = n.child(ci) {
                        if c.kind() == "simple_identifier" || c.kind() == "identifier" {
                            let name = c.utf8_text(bytes).unwrap_or("").to_string();
                            if !name.is_empty() {
                                table.add_symbol(
                                    name,
                                    SymbolKind::Property,
                                    Visibility::Implicit,
                                    pos.row + 1,
                                    pos.column + 1,
                                    sid,
                                );
                            }
                            break;
                        }
                    }
                }
                push_children(&n, &mut stack, sid);
            }

            // ── Local variables (val/var inside functions) ──
            "variable_declaration" => {
                let pos = n.start_position();
                if let Some(name) = extract_property_name(&n, bytes) {
                    table.add_symbol(
                        name,
                        SymbolKind::Property,
                        Visibility::Implicit,
                        pos.row + 1,
                        pos.column + 1,
                        sid,
                    );
                }
                push_children(&n, &mut stack, sid);
            }

            // ── Enum constants ──
            "enum_entry" => {
                for ci in 0..n.child_count() {
                    if let Some(c) = n.child(ci) {
                        if c.kind() == "simple_identifier" || c.kind() == "identifier" {
                            let name = c.utf8_text(bytes).unwrap_or("").to_string();
                            if !name.is_empty() {
                                table.add_symbol(
                                    name,
                                    SymbolKind::Property,
                                    Visibility::Implicit,
                                    c.start_position().row + 1,
                                    c.start_position().column + 1,
                                    sid,
                                );
                            }
                            break;
                        }
                    }
                }
                push_children(&n, &mut stack, sid);
            }

            _ => {
                push_children(&n, &mut stack, sid);
            }
        }
    }
}

fn extract_class_symbol(node: &Node, bytes: &[u8]) -> Option<ClassInfo> {
    let kind = node.kind();
    let sym_kind = match kind {
        "object_declaration" => SymbolKind::Object,
        "enum_class" => SymbolKind::Enum,
        _ => SymbolKind::Class,
    };

    let vis = extract_visibility(node, bytes);
    let pos = node.start_position();

    // Find the name (simple_identifier) among children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "type_identifier"
                || child.kind() == "simple_identifier"
                || child.kind() == "identifier"
            {
                let name = child.utf8_text(bytes).unwrap_or("").to_string();
                return Some(ClassInfo {
                    name,
                    kind: sym_kind,
                    visibility: vis,
                    line: pos.row + 1,
                    col: pos.column + 1,
                });
            }
            // For object declarations, the name is the object keyword if unnamed
            if kind == "object_declaration" && child.kind() == "object" {
                // Check if next child is an identifier (named object) or not (anonymous)
                for j in (i + 1)..node.child_count() {
                    if let Some(nc) = node.child(j) {
                        if nc.kind() == "simple_identifier" || nc.kind() == "identifier" {
                            let name = nc.utf8_text(bytes).unwrap_or("").to_string();
                            return Some(ClassInfo {
                                name,
                                kind: sym_kind,
                                visibility: vis,
                                line: pos.row + 1,
                                col: pos.column + 1,
                            });
                        }
                        break;
                    }
                }
            }
            // For companion objects
            if kind == "object_declaration" && child.kind() == "companion" {
                return Some(ClassInfo {
                    name: "Companion".into(),
                    kind: SymbolKind::Object,
                    visibility: vis,
                    line: pos.row + 1,
                    col: pos.column + 1,
                });
            }
        }
    }
    None
}

struct ClassInfo {
    name: String,
    kind: SymbolKind,
    visibility: Visibility,
    line: usize,
    col: usize,
}

fn extract_function_symbol(node: &Node, bytes: &[u8]) -> Option<ClassInfo> {
    let vis = extract_visibility(node, bytes);
    let pos = node.start_position();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "simple_identifier" || child.kind() == "identifier" {
                let name = child.utf8_text(bytes).unwrap_or("").to_string();
                return Some(ClassInfo {
                    name,
                    kind: SymbolKind::Function,
                    visibility: vis,
                    line: pos.row + 1,
                    col: pos.column + 1,
                });
            }
        }
    }
    None
}

fn extract_property_name(node: &Node, bytes: &[u8]) -> Option<String> {
    // val/var name: property_declaration → val/var → identifier
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "simple_identifier" || child.kind() == "identifier" {
                return Some(child.utf8_text(bytes).unwrap_or("").to_string());
            }
            // Skip val/var keywords
        }
    }
    None
}

fn extract_visibility(node: &Node, bytes: &[u8]) -> Visibility {
    // Check modifiers for visibility keywords
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "modifiers" {
                let text = child.utf8_text(bytes).unwrap_or("");
                if text.contains("private") {
                    return Visibility::Private;
                }
                if text.contains("protected") {
                    return Visibility::Protected;
                }
                if text.contains("internal") {
                    return Visibility::Internal;
                }
                if text.contains("public") {
                    return Visibility::Public;
                }
            }
            // Direct visibility keywords (some grammars put them as direct children)
            if child.kind() == "private" || child.kind() == "private_modifier" {
                return Visibility::Private;
            }
            if child.kind() == "protected" || child.kind() == "protected_modifier" {
                return Visibility::Protected;
            }
            if child.kind() == "internal" || child.kind() == "internal_modifier" {
                return Visibility::Internal;
            }
        }
    }
    Visibility::Implicit
}

fn extract_imports(node: &Node, bytes: &[u8], table: &mut SymbolTable) {
    // import_header → "import" + identifier (possibly with wildcard)
    let text = node.utf8_text(bytes).unwrap_or("");
    let trimmed = text.strip_prefix("import").unwrap_or(&text).trim();

    if trimmed.ends_with(".*") {
        let pkg = &trimmed[..trimmed.len() - 2];
        table.add_star_import(pkg.to_string());
    } else {
        // Named import: "com.example.Foo"
        if let Some(last) = trimmed.rsplit('.').next() {
            table.add_import(last.to_string(), trimmed.to_string());
        }
    }
}

fn push_children<'a>(node: &Node<'a>, stack: &mut Vec<(Node<'a>, usize)>, scope_id: usize) {
    for i in (0..node.child_count()).rev() {
        if let Some(child) = node.child(i) {
            stack.push((child, scope_id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;

    fn build(src: &str) -> SymbolTable {
        let mut p = KotlinParser::new();
        let tree = p.parse(src);
        build_symbol_table(src, tree.root_node())
    }

    #[test]
    fn basic_class() {
        let t = build("class Foo { fun bar() {} }");
        let syms: Vec<_> = t.symbols.iter().map(|s| s.name.clone()).collect();
        assert!(syms.contains(&"Foo".into()), "symbols: {:?}", syms);
        assert!(syms.contains(&"bar".into()), "symbols: {:?}", syms);
    }

    #[test]
    fn private_visibility() {
        let t = build("class Foo { private fun bar() {} }");
        let bar = t.symbols.iter().find(|s| s.name == "bar").unwrap();
        assert_eq!(bar.visibility, Visibility::Private);
    }

    #[test]
    fn imports() {
        let t = build("import com.example.Foo\nclass Bar { val x: Foo = Foo() }");
        assert_eq!(t.imports.get("Foo"), Some(&"com.example.Foo".into()));
    }
}
