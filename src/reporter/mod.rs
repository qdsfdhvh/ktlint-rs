//! Diagnostic reporters — formats violations for human or machine consumption.

use crate::cli::Cli;
use crate::rules::Violation;
use colored::Colorize;

/// Report violations in the selected format.
pub struct DiagnosticReporter<'a> {
    cli: &'a Cli,
}

impl<'a> DiagnosticReporter<'a> {
    pub fn new(cli: &'a Cli) -> Self {
        Self { cli }
    }

    /// Print all violations and return the exit code (0 = no violations, 1 = violations found).
    pub fn report(&self, violations: &[Violation]) -> i32 {
        match self.cli.reporter.as_str() {
            "json" => self.report_json(violations),
            "sarif" => self.report_sarif(violations),
            "plain-summary" => self.report_plain_summary(violations),
            _ => self.report_plain(violations),
        }
    }

    fn report_plain(&self, violations: &[Violation]) -> i32 {
        if violations.is_empty() {
            return 0;
        }

        let path_width = violations
            .iter()
            .map(|v| {
                if self.cli.relative {
                    v.file.len()
                } else {
                    v.file.len()
                }
            })
            .max()
            .unwrap_or(20);

        for v in violations {
            let path = &v.file;
            let location = format!("{:>width$}:{}:{}", path, v.line, v.col, width = path_width);
            let rule = format!("({})", v.rule_id).dimmed();
            let msg = &v.message;

            if self.cli.color {
                println!("{} {} {}", location.red(), rule, msg.yellow());
            } else {
                println!("{} {} {}", location, rule, msg);
            }
        }

        // Summary
        if self.cli.format {
            eprintln!(
                "\n{}",
                "Lint has found errors that can be autocorrected using 'ktlint --format'".yellow()
            );
        }
        self.print_summary(violations);
        1
    }

    fn report_json(&self, violations: &[Violation]) -> i32 {
        let output: Vec<serde_json::Value> = violations
            .iter()
            .map(|v| {
                serde_json::json!({
                    "file": v.file,
                    "line": v.line,
                    "col": v.col,
                    "rule": v.rule_id,
                    "message": v.message,
                    "auto_fixable": v.auto_fixable,
                })
            })
            .collect();

        let json = serde_json::to_string_pretty(&output).unwrap_or_default();
        println!("{}", json);
        if violations.is_empty() {
            0
        } else {
            1
        }
    }

    fn report_sarif(&self, violations: &[Violation]) -> i32 {
        // Minimal SARIF v2.1.0 output
        let results: Vec<serde_json::Value> = violations
            .iter()
            .map(|v| {
                serde_json::json!({
                    "ruleId": v.rule_id,
                    "message": { "text": v.message },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": { "uri": v.file },
                            "region": { "startLine": v.line, "startColumn": v.col }
                        }
                    }]
                })
            })
            .collect();

        let sarif = serde_json::json!({
            "$schema": "https://json.schemastore.org/sarif-2.1.0-rtm.5.json",
            "version": "2.1.0",
            "runs": [{ "results": results }]
        });

        println!(
            "{}",
            serde_json::to_string_pretty(&sarif).unwrap_or_default()
        );
        if violations.is_empty() {
            0
        } else {
            1
        }
    }

    fn report_plain_summary(&self, violations: &[Violation]) -> i32 {
        self.print_summary(violations);
        if violations.is_empty() {
            0
        } else {
            1
        }
    }

    fn print_summary(&self, violations: &[Violation]) {
        if violations.is_empty() {
            return;
        }

        // Group by rule_id and count
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for v in violations {
            *counts.entry(&v.rule_id).or_default() += 1;
        }
        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        eprintln!("\nSummary error count (descending) by rule:");
        for (rule, count) in &sorted {
            eprintln!("  {}: {}", rule, count);
        }
    }
}
