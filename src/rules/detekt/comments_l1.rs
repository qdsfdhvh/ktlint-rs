//! detekt comments L1 rules — KDoc checks that need declaration/visibility info.
//! SymbolTable provided by the engine; shared across all L1 rules.

use crate::resolver::{SymbolKind, Visibility};
use crate::rules::{Rule, Violation};
use std::collections::HashSet;
use tree_sitter::{Node, Tree};

// ── KDocReferencesNonPublicProperty ──

pub struct KDocReferencesNonPublicProperty;

impl Rule for KDocReferencesNonPublicProperty {
    fn id(&self) -> &'static str {
        "detekt:comments:KDocReferencesNonPublicProperty"
    }
    fn auto_fixable(&self) -> bool {
        false
    }

    fn check_with_symbols(
        &self,
        tree: &Tree,
        source: &str,
        sym: Option<&crate::resolver::SymbolTable>,
    ) -> Vec<Violation> {
        let table = sym.expect("SymbolTable should be provided by engine");
        let mut non_public: HashSet<String> = table
            .symbols
            .iter()
            .filter(|s| {
                s.kind == SymbolKind::Property
                    && matches!(s.visibility, Visibility::Private | Visibility::Protected)
            })
            .map(|s| s.name.clone())
            .collect();
        // Constructor properties (class_parameter with non-public val/var)
        collect_non_public_ctor_props(tree.root_node(), source.as_bytes(), &mut non_public);

        let mut v = Vec::new();
        let mut in_kdoc = false;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("/**") {
                in_kdoc = true;
            }
            if in_kdoc {
                for name in bracket_refs(t) {
                    if non_public.contains(&name) {
                        v.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: "detekt:comments:KDocReferencesNonPublicProperty".into(),
                            message: format!("KDoc references non-public property '{}'", name),
                            auto_fixable: false,
                        });
                    }
                }
                if t.ends_with("*/") {
                    in_kdoc = false;
                }
            }
        }
        v
    }

    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        {
            use crate::resolver::builder::build_symbol_table;
            let sym = build_symbol_table(source, tree.root_node());
            self.check_with_symbols(tree, source, Some(&sym))
        }
    }
}

fn bracket_refs(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('[') {
        let after = &rest[start + 1..];
        match after.find(']') {
            Some(end) => {
                let name = &after[..end];
                if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    out.push(name.to_string());
                }
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    out
}

fn collect_non_public_ctor_props(n: Node, bytes: &[u8], out: &mut HashSet<String>) {
    if n.kind() == "class_parameter" {
        let text = n.utf8_text(bytes).unwrap_or("");
        let is_prop = text.contains("val ") || text.contains("var ");
        let is_non_public = text.starts_with("private") || text.starts_with("protected");
        if is_prop && is_non_public {
            for i in 0..n.child_count() {
                if let Some(c) = n.child(i) {
                    if c.kind() == "simple_identifier" {
                        out.insert(c.utf8_text(bytes).unwrap_or("").to_string());
                        break;
                    }
                }
            }
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            collect_non_public_ctor_props(c, bytes, out);
        }
    }
}

// ── OutdatedDocumentation ──

pub struct OutdatedDocumentation;

impl Rule for OutdatedDocumentation {
    fn id(&self) -> &'static str {
        "detekt:comments:OutdatedDocumentation"
    }
    fn auto_fixable(&self) -> bool {
        false
    }

    fn check_with_symbols(
        &self,
        tree: &Tree,
        source: &str,
        __sym: Option<&crate::resolver::SymbolTable>,
    ) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        let mut stack = vec![tree.root_node()];
        while let Some(n) = stack.pop() {
            if n.kind() == "multiline_comment" {
                check_kdoc_params(&n, bytes, &mut v);
            }
            for i in (0..n.child_count()).rev() {
                if let Some(c) = n.child(i) {
                    stack.push(c);
                }
            }
        }
        v
    }

    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        {
            use crate::resolver::builder::build_symbol_table;
            let sym = build_symbol_table(source, tree.root_node());
            self.check_with_symbols(tree, source, Some(&sym))
        }
    }
}

fn check_kdoc_params(comment: &Node, bytes: &[u8], v: &mut Vec<Violation>) {
    let text = comment.utf8_text(bytes).unwrap_or("");
    if !text.starts_with("/**") || !text.contains("@param") {
        return;
    }
    let mut sib = comment.next_sibling();
    while let Some(s) = sib {
        match s.kind() {
            "comment" | "multiline_comment" => sib = s.next_sibling(),
            "class_declaration" | "function_declaration" => {
                let params = decl_param_names(&s, bytes);
                let start_line = comment.start_position().row;
                for (off, line) in text.lines().enumerate() {
                    let t = line.trim().trim_start_matches('*').trim();
                    if let Some(rest) = t.strip_prefix("@param ") {
                        let name = rest.split_whitespace().next().unwrap_or("");
                        if !name.is_empty() && !params.contains(&name.to_string()) {
                            v.push(Violation {
                                file: String::new(),
                                line: start_line + off + 1,
                                col: 1,
                                rule_id: "detekt:comments:OutdatedDocumentation".into(),
                                message: format!(
                                    "@param '{}' does not match any declared parameter",
                                    name
                                ),
                                auto_fixable: false,
                            });
                        }
                    }
                }
                return;
            }
            _ => return,
        }
    }
}

fn decl_param_names(decl: &Node, bytes: &[u8]) -> Vec<String> {
    let mut params = Vec::new();
    let mut stack = vec![*decl];
    while let Some(n) = stack.pop() {
        if matches!(n.kind(), "class_parameter" | "parameter") {
            for i in 0..n.child_count() {
                if let Some(c) = n.child(i) {
                    if c.kind() == "simple_identifier" {
                        params.push(c.utf8_text(bytes).unwrap_or("").to_string());
                        break;
                    }
                }
            }
        }
        if n.kind() == "class_body" || n.kind() == "function_body" {
            continue;
        }
        for i in (0..n.child_count()).rev() {
            if let Some(c) = n.child(i) {
                stack.push(c);
            }
        }
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn kdoc_ref(s: &str) -> Vec<Violation> {
        KDocReferencesNonPublicProperty.check(&KotlinParser::new().parse(s), s)
    }
    fn outdated(s: &str) -> Vec<Violation> {
        OutdatedDocumentation.check(&KotlinParser::new().parse(s), s)
    }
    #[test]
    fn kdoc_ref_private_bad() {
        assert!(!kdoc_ref(
            "/**\n * Uses [secret] here.\n */\nclass Foo { private val secret = 1 }\n"
        )
        .is_empty());
    }
    #[test]
    fn kdoc_ref_ctor_private_bad() {
        assert!(
            !kdoc_ref("/**\n * Uses [token].\n */\nclass Auth(private val token: String)\n")
                .is_empty()
        );
    }
    #[test]
    fn kdoc_ref_public_ok() {
        assert!(kdoc_ref("/**\n * Uses [name].\n */\nclass Foo { val name = \"x\" }\n").is_empty());
    }
    #[test]
    fn kdoc_ref_no_refs_ok() {
        assert!(kdoc_ref("/** Plain doc. */\nclass Foo { private val secret = 1 }\n").is_empty());
    }
    #[test]
    fn outdated_param_bad() {
        assert!(
            !outdated("/**\n * @param wrong desc\n */\nfun f(right: Int) = right\n").is_empty()
        );
    }
    #[test]
    fn outdated_param_ok() {
        assert!(outdated("/**\n * @param right desc\n */\nfun f(right: Int) = right\n").is_empty());
    }
    #[test]
    fn outdated_class_param_ok() {
        assert!(outdated("/**\n * @param a doc\n */\nclass Foo(val a: Int)\n").is_empty());
    }
    #[test]
    fn outdated_class_param_bad() {
        assert!(!outdated("/**\n * @param b doc\n */\nclass Foo(val a: Int)\n").is_empty());
    }
    #[test]
    fn no_kdoc_ok() {
        assert!(outdated("fun f(x: Int) = x\n").is_empty());
    }
}
