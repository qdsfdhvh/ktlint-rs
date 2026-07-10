//! standard:modifier-order — enforce Kotlin modifier ordering.
//!
//! Correct order: public/protected/private → expect/actual → final/open/abstract/sealed/const
//! → external → override → lateinit → tailrec → vararg → suspend → inner → enum/annotation/fun
//! → companion → inline → value → operator → infix → data → reified
//!
//! Simplified: visibility → inheritance → misc → keyword

use crate::rules::{Rule, Violation};

const MODIFIER_ORDER: &[&str] = &[
    "public", "protected", "private", "internal",
    "expect", "actual",
    "final", "open", "abstract", "sealed", "const",
    "external",
    "override",
    "lateinit",
    "tailrec",
    "vararg",
    "suspend",
    "inner",
    "companion",
    "inline",
    "value",
    "operator",
    "infix",
    "data",
    "crossinline",
    "noinline",
    "reified",
];

pub struct ModifierOrder;

impl Rule for ModifierOrder {
    fn id(&self) -> &'static str {
        "standard:modifier-order"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // Extract modifier keywords from the line
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            let modifiers: Vec<(&str, usize)> = words
                .iter()
                .enumerate()
                .filter(|(_, w)| MODIFIER_ORDER.contains(w))
                .map(|(idx, w)| (*w, idx))
                .collect();

            if modifiers.len() < 2 {
                continue;
            }

            // Check that modifiers are in the correct order
            for j in 0..modifiers.len() - 1 {
                let current = modifiers[j].0;
                let next = modifiers[j + 1].0;

                let current_pos = MODIFIER_ORDER.iter().position(|&m| m == current);
                let next_pos = MODIFIER_ORDER.iter().position(|&m| m == next);

                if let (Some(cp), Some(np)) = (current_pos, next_pos) {
                    if cp > np {
                        // Out of order: `next` should come before `current`
                        violations.push(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: self.id().to_string(),
                            message: format!(
                                "Modifier order violation: \"{}\" should come before \"{}\"",
                                next, current
                            ),
                            auto_fixable: true,
                        });
                        break; // One violation per line
                    }
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
        ModifierOrder.check(&tree, source)
    }

    #[test]
    fn correct_modifier_order() {
        assert!(check("public override fun foo()\n").is_empty());
        assert!(check("private suspend fun bar()\n").is_empty());
        assert!(check("inline fun baz()\n").is_empty());
    }

    #[test]
    fn incorrect_modifier_order() {
        let v = check("override public fun foo()\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:modifier-order");
    }

    #[test]
    fn no_modifiers() {
        assert!(check("fun foo()\n").is_empty());
    }

    #[test]
    fn data_class_correct() {
        assert!(check("data class Foo(val x: Int)\n").is_empty());
    }
}
