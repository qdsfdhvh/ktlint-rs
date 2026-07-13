//! detekt potential-bugs rules — catch common mistakes. L0, CST/text level.
use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

// ── DuplicateCaseInWhen ──
pub struct DuplicateCaseInWhen;
impl Rule for DuplicateCaseInWhen {
    fn id(&self) -> &'static str { "detekt:potential-bugs:DuplicateCaseInWhen" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_when_entries(tree.root_node(), source.as_bytes(), &mut v);
        v
    }
}

fn walk_when_entries(n: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
    if n.kind() == "when_expression" {
        let mut seen: Vec<String> = Vec::new();
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i) {
                if c.kind() == "when_entry" {
                    // Get the condition text
                    for j in 0..c.child_count() {
                        if let Some(cond) = c.child(j) {
                            let k = cond.kind();
                            if k != "when_entry" && k != "control_structure_body" && k != "->" {
                                let text = cond.utf8_text(bytes).unwrap_or("").trim().to_string();
                                if !text.is_empty() && !text.is_empty() {
                                    if seen.contains(&text) {
                                        let pos = cond.start_position();
                                        v.push(Violation {
                                            file: String::new(), line: pos.row+1, col: pos.column+1,
                                            rule_id: "detekt:potential-bugs:DuplicateCaseInWhen".into(),
                                            message: format!("Duplicate case in when: {}", text),
                                            auto_fixable: false,
                                        });
                                    } else { seen.push(text); }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { walk_when_entries(c, bytes, v); }
    }
}

// ── UnreachableCatchBlock ──
pub struct UnreachableCatchBlock;
impl Rule for UnreachableCatchBlock {
    fn id(&self) -> &'static str { "detekt:potential-bugs:UnreachableCatchBlock" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_catch(tree.root_node(), source.as_bytes(), &mut v);
        v
    }
}

fn walk_catch(n: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
    if n.kind() == "try_expression" {
        let mut prev_types: Vec<String> = Vec::new();
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i) {
                if c.kind() == "catch_block" {
                    let mut ex_type = String::new();
                    for j in 0..c.child_count() {
                        if let Some(tn) = c.child(j) {
                            if tn.kind() == "user_type" || tn.kind() == "type_identifier" {
                                if let Ok(txt) = tn.utf8_text(bytes) {
                                    ex_type = txt.to_string();
                                }
                            }
                        }
                    }
                    if !ex_type.is_empty() {
                        if prev_types.contains(&ex_type) {
                            let pos = c.start_position();
                            v.push(Violation {
                                file: String::new(), line: pos.row+1, col: pos.column+1,
                                rule_id: "detekt:potential-bugs:UnreachableCatchBlock".into(),
                                message: format!("Catch block for {} is unreachable", ex_type),
                                auto_fixable: false,
                            });
                        }
                        prev_types.push(ex_type);
                    }
                }
            }
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { walk_catch(c, bytes, v); }
    }
}

// ── EqualsNullCall ──
pub struct EqualsNullCall;
impl Rule for EqualsNullCall {
    fn id(&self) -> &'static str { "detekt:potential-bugs:EqualsNullCall" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _t: &Tree, s: &str) -> Vec<Violation> {
        s.lines().enumerate().filter_map(|(i, l)| {
            if l.contains(".equals(null)") || l.contains(".equals (null)") {
                Some(Violation {
                    file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:potential-bugs:EqualsNullCall".into(),
                    message: "Use == null instead of .equals(null)".into(),
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

    #[test] fn dup_when_bad() {
        assert!(!c(&DuplicateCaseInWhen, "fun f(){when(x){1->{} 1->{}}}\n").is_empty());
    }
    #[test] fn dup_when_ok() {
        assert!(c(&DuplicateCaseInWhen, "fun f(){when(x){1->{} 2->{}}}\n").is_empty());
    }
    #[test] fn equals_null() {
        assert!(!c(&EqualsNullCall, "x.equals(null)\n").is_empty());
    }
}
