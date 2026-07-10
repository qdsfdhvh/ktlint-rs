//! @Suppress / @SuppressWarnings annotation support.
//!
//! Handles per-element rule suppression via annotations:
//! - `@Suppress("ktlint")` — suppress all ktlint rules on this element
//! - `@Suppress("ktlint:standard:curly-spacing")` — suppress a specific rule
//! - `@SuppressWarnings("ktlint")` — same as @Suppress
//! - Multiple args: `@Suppress("ktlint:rule1", "ktlint:rule2")`

use std::collections::HashSet;

/// Parse @Suppress annotations from source code and return the set of
/// suppressed rule IDs for each line range.
///
/// Returns vec of (line, rule_id) pairs where the rule is suppressed.
/// If `rule_id` is empty string, ALL rules are suppressed on that line.
pub fn parse_suppress_annotations(source: &str) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Match @Suppress or @SuppressWarnings
        if trimmed.starts_with("@Suppress(") || trimmed.starts_with("@SuppressWarnings(") {
            // Extract the content inside the parentheses
            if let Some(args_start) = trimmed.find('(') {
                if let Some(args_end) = trimmed[args_start..].find(')') {
                    let args = &trimmed[args_start + 1..args_start + args_end];

                    // Split by comma, trim quotes
                    for arg in args.split(',') {
                        let arg = arg.trim().trim_matches('"').trim();
                        if arg == "ktlint" {
                            // Suppress ALL rules on the NEXT line (annotated element)
                            results.push((i + 2, String::new()));
                        } else if let Some(rule) = arg.strip_prefix("ktlint:") {
                            results.push((i + 2, rule.to_string()));
                        }
                    }
                }
            }
        }
    }

    results
}

/// Filter out violations that are suppressed by @Suppress annotations.
pub fn filter_suppressed(violations: Vec<crate::rules::Violation>, source: &str) -> Vec<crate::rules::Violation> {
    let suppressed = parse_suppress_annotations(source);

    // Build a set of suppressed rule IDs per line
    let mut fully_suppressed_lines: HashSet<usize> = HashSet::new();
    let mut rule_suppressions: Vec<(usize, String)> = Vec::new();

    for (line, rule_id) in &suppressed {
        if rule_id.is_empty() {
            fully_suppressed_lines.insert(*line);
        } else {
            rule_suppressions.push((*line, rule_id.clone()));
        }
    }

    violations
        .into_iter()
        .filter(|v| {
            // Fully suppressed lines
            if fully_suppressed_lines.contains(&v.line) {
                return false;
            }
            // Per-rule suppression
            for (line, rule_id) in &rule_suppressions {
                if *line == v.line && v.rule_id == *rule_id {
                    return false;
                }
            }
            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Violation;

    #[test]
    fn parse_single_suppress() {
        let source = "@Suppress(\"ktlint:standard:curly-spacing\")\nclass Foo\n";
        let s = parse_suppress_annotations(source);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0], (2, "standard:curly-spacing".to_string()));
    }

    #[test]
    fn parse_ktlint_suppress_all() {
        let source = "@Suppress(\"ktlint\")\nfun foo()\n";
        let s = parse_suppress_annotations(source);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].1, "");         assert_eq!(s[0].0, 2); // line 2 (annotated element),
        assert_eq!(s[0].1, ""); // empty = all rules
    }

    #[test]
    fn filter_suppressed_violations() {
        let source = "@Suppress(\"ktlint:standard:curly-spacing\")\nclass Foo{\n}\n";
        let violations = vec![
            Violation {
                file: String::new(),
                line: 2, // class Foo line (annotated element)
                col: 1,
                rule_id: "standard:curly-spacing".to_string(),
                message: "Missing space".to_string(),
                auto_fixable: true,
            },
            Violation {
                file: String::new(),
                line: 2,
                col: 1,
                rule_id: "standard:op-spacing".to_string(),
                message: "Missing space".to_string(),
                auto_fixable: true,
            },
        ];
        let filtered = filter_suppressed(violations, source);
        assert_eq!(filtered.len(), 1); // curly-spacing filtered, op-spacing remains
        assert_eq!(filtered[0].rule_id, "standard:op-spacing");
    }

    #[test]
    fn suppres_warnings_works() {
        let source = "@SuppressWarnings(\"ktlint\")\nfun foo()\n";
        let s = parse_suppress_annotations(source);
        assert_eq!(s.len(), 1);
    }
}
