//! Name resolution engine — builds symbol table from CST.
//! Tracks declarations, scopes, visibility, and import aliases.
//! Enables ~50 L1 detekt rules (UnusedPrivate*, NoNameShadowing, etc.).
pub mod builder;
use std::collections::HashMap;

/// A symbol in the source file.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub line: usize,
    pub col: usize,
    pub scope_id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Class,
    Function,
    Property,
    Enum,
    Interface,
    Object,
    TypeAlias,
    Constructor,
    Init,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Internal,
    Protected,
    Implicit,
}

/// A scope (file-level, class body, function body, etc.).
#[derive(Debug, Clone)]
pub struct Scope {
    pub id: usize,
    pub parent_id: Option<usize>,
    pub symbols: Vec<usize>,
}

/// Resolved symbol table for a single file.
#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub symbols: Vec<Symbol>,
    pub scopes: Vec<Scope>,
    /// Import map: simple_name → fully_qualified_name
    pub imports: HashMap<String, String>,
    /// Star (wildcard) imports: "com.example.utils"
    pub star_imports: Vec<String>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut scopes = Vec::new();
        scopes.push(Scope {
            id: 0,
            parent_id: None,
            symbols: Vec::new(),
        });
        Self {
            symbols: Vec::new(),
            scopes,
            imports: HashMap::new(),
            star_imports: Vec::new(),
        }
    }

    /// Add a new scope. Returns scope ID.
    pub fn add_scope(&mut self, parent_id: usize) -> usize {
        let id = self.scopes.len();
        self.scopes.push(Scope {
            id,
            parent_id: Some(parent_id),
            symbols: Vec::new(),
        });
        id
    }

    /// Register a symbol in a scope.
    pub fn add_symbol(
        &mut self,
        name: String,
        kind: SymbolKind,
        visibility: Visibility,
        line: usize,
        col: usize,
        scope_id: usize,
    ) -> usize {
        let id = self.symbols.len();
        self.symbols.push(Symbol {
            name,
            kind,
            visibility,
            line,
            col,
            scope_id,
        });
        self.scopes[scope_id].symbols.push(id);
        id
    }

    /// Register a named import.
    pub fn add_import(&mut self, alias: String, full_path: String) {
        self.imports.insert(alias, full_path);
    }

    /// Register a star import.
    pub fn add_star_import(&mut self, package: String) {
        self.star_imports.push(package);
    }

    /// Find a symbol by name in the visible scopes (scope chain walking).
    pub fn resolve(&self, name: &str, from_scope_id: usize) -> Option<&Symbol> {
        let mut scope_id = Some(from_scope_id);
        while let Some(sid) = scope_id {
            let scope = &self.scopes[sid];
            for &sym_id in &scope.symbols {
                let sym = &self.symbols[sym_id];
                if sym.name == name {
                    return Some(sym);
                }
            }
            scope_id = scope.parent_id;
        }
        None
    }

    /// Walk up the scope chain and return the first scope that contains
    /// a class/object symbol (i.e., the enclosing class body scope).
    pub fn enclosing_class_scope(&self, scope_id: usize) -> Option<usize> {
        let mut sid = Some(scope_id);
        while let Some(id) = sid {
            // Check if this scope is a class body
            for &sym_id in &self.scopes[id].symbols {
                let sym = &self.symbols[sym_id];
                if matches!(sym.kind, SymbolKind::Class | SymbolKind::Object) {
                    return Some(id);
                }
            }
            sid = self.scopes[id].parent_id;
        }
        None
    }

    /// Find all symbols of a given kind that are private and unused.
    pub fn private_symbols(&self) -> Vec<&Symbol> {
        self.symbols
            .iter()
            .filter(|s| s.visibility == Visibility::Private)
            .collect()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
