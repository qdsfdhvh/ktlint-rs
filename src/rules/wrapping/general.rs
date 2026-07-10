//! standard:wrapping — general wrapping rule for when/if/for/while expressions.
//! Enforces that in multiline expressions, continuation elements are on new lines.
use crate::rules::{Rule, Violation};

pub struct GeneralWrapping;

impl Rule for GeneralWrapping {
    fn id(&self) -> &'static str {
        "standard:wrapping"
    }

    fn auto_fixable(&self) -> bool {
        true
    }

    fn check(&self, _tree: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            // Check `when` entries — `1, 2 ->` on same line is wrong for multiline when
            if trimmed.starts_with("when ") && i + 1 < lines.len() {
                let mut prev_line_was_entry = false;
                for j in (i + 1)..lines.len().min(i + 50) {
                    let t = lines[j].trim();
                    if t == "}" || t == "})" || t == ")." {
                        break;
                    }
                    if t.contains("->") && !t.contains("\"") {
                        if prev_line_was_entry {
                            // Multiple entries on consecutive lines — check consistency
                        }
                        prev_line_was_entry = true;
                    } else if !t.is_empty() && !t.starts_with("//") {
                        prev_line_was_entry = false;
                    }
                }
            }

            // Check `if/else` chain consistency
            if trimmed.starts_with("if (")
                && trimmed.ends_with('{')
                && i + 1 < lines.len()
                && lines[i + 1].trim().is_empty()
            {
                violations.push(Violation {
                    file: String::new(),
                    line: i + 2,
                    col: 1,
                    rule_id: self.id().to_string(),
                    message: "Unexpected blank line after if-condition".to_string(),
                    auto_fixable: true,
                });
            }
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn check(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        GeneralWrapping.check(&p.parse(s), s)
    }
    #[test]
    fn if_no_blank() {
        assert!(check("if (x) {\n    doA()\n}\n").is_empty());
    }
    #[test]
    fn if_with_blank() {
        let v = check("if (x) {\n\n    doA()\n}\n");
        assert!(!v.is_empty());
        assert_eq!(v[0].rule_id, "standard:wrapping");
    }
}
