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
            "html" => self.report_html(violations),
            "xml" | "checkstyle" => self.report_checkstyle_xml(violations),
            "markdown" | "md" => self.report_markdown(violations),
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

    fn report_html(&self, violations: &[Violation]) -> i32 {
        let mut html = String::from(
            "<!DOCTYPE html>\n<html><head><meta charset=\"UTF-8\">\
             <title>ktlint-rs Report</title>\n<style>\n\
             body{font-family:monospace;margin:20px;background:#0d1117;color:#c9d1d9}\n\
             h1{color:#58a6ff}.violation{margin:4px 0;padding:8px;border-left:3px solid #f85149;background:#161b22}\n\
             .file{color:#8b949e}.rule{color:#d2a8ff}.line{color:#ffa657}.msg{color:#f85149}\n\
             .summary{color:#7ee787}th,td{text-align:left;padding:4px 12px}\n\
             </style></head><body><h1>ktlint-rs Report</h1>\n",
        );
        if violations.is_empty() {
            html.push_str("<p>No violations found.</p>\n");
        } else {
            html.push_str(&format!("<p>{} violations found.</p>\n", violations.len()));
            // Summary table
            let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
            for v in violations { *counts.entry(&v.rule_id).or_default() += 1; }
            let mut sorted: Vec<_> = counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
            html.push_str("<table class=\"summary\"><tr><th>Rule</th><th>Count</th></tr>\n");
            for (rule, count) in &sorted {
                html.push_str(&format!("<tr><td>{}</td><td>{}</td></tr>\n", rule, count));
            }
            html.push_str("</table>\n");
            // Violation list
            for v in violations {
                html.push_str(&format!(
                    "<div class=\"violation\">\
                     <span class=\"file\">{}</span>\
                     <span class=\"line\">:{}:{}</span>\
                     <span class=\"rule\">({})</span>\
                     <span class=\"msg\">{}</span></div>\n",
                    v.file, v.line, v.col, v.rule_id, v.message
                ));
            }
        }
        html.push_str("</body></html>\n");
        println!("{}", html);
        self.print_summary(violations);
        if violations.is_empty() { 0 } else { 1 }
    }

    fn report_checkstyle_xml(&self, violations: &[Violation]) -> i32 {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<checkstyle version=\"1.0\">\n");
        // Group by file
        let mut files: std::collections::HashMap<&str, Vec<&Violation>> = std::collections::HashMap::new();
        for v in violations { files.entry(&v.file).or_default().push(v); }
        let mut file_names: Vec<&&str> = files.keys().collect();
        file_names.sort();
        for file_name in file_names {
            xml.push_str(&format!("<file name=\"{}\">\n", xml_attr_escape(file_name)));
            for v in &files[file_name] {
                xml.push_str(&format!(
                    "<error line=\"{}\" column=\"{}\" severity=\"error\" \
                     message=\"{}\" source=\"{}\"/>\n",
                    v.line, v.col,
                    xml_attr_escape(&v.message),
                    xml_attr_escape(&v.rule_id)
                ));
            }
            xml.push_str("</file>\n");
        }
        xml.push_str("</checkstyle>\n");
        println!("{}", xml);
        self.print_summary(violations);
        if violations.is_empty() { 0 } else { 1 }
    }

    fn report_markdown(&self, violations: &[Violation]) -> i32 {
        let mut md = String::from("# ktlint-rs Report\n\n");
        if violations.is_empty() {
            md.push_str("No violations found.\n");
        } else {
            md.push_str(&format!("**{} violations** found.\n\n", violations.len()));
            md.push_str("| Rule | Count |\n|------|-------|\n");
            let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
            for v in violations { *counts.entry(&v.rule_id).or_default() += 1; }
            let mut sorted: Vec<_> = counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
            for (rule, count) in &sorted {
                md.push_str(&format!("| {} | {} |\n", rule, count));
            }
            md.push_str("\n## Violations\n\n");
            md.push_str("| File | Line | Rule | Message |\n|------|------|------|--------|\n");
            for v in violations {
                md.push_str(&format!(
                    "| {} | {}:{} | {} | {} |\n",
                    v.file, v.line, v.col, v.rule_id, v.message
                ));
            }
        }
        println!("{}", md);
        self.print_summary(violations);
        if violations.is_empty() { 0 } else { 1 }
    }
}

fn xml_attr_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
