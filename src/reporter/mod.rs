//! Diagnostic reporters — formats violations for human or machine consumption.

use crate::cli::Cli;
use crate::rules::Violation;
use colored::Colorize;
use std::io::Write;

/// Report violations in the selected format.
/// Supports `--limit`, `--reporter-output`, and `--relative`.
pub struct DiagnosticReporter<'a> {
    cli: &'a Cli,
    /// Current working directory for `--relative` path stripping.
    cwd: String,
}

impl<'a> DiagnosticReporter<'a> {
    pub fn new(cli: &'a Cli) -> Self {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        Self { cli, cwd }
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

    /// Apply `--limit`: truncate violations to the first N.
    fn limited<'v>(&self, violations: &'v [Violation]) -> &'v [Violation] {
        match self.cli.limit {
            Some(n) if n > 0 => {
                if n < violations.len() {
                    &violations[..n]
                } else {
                    violations
                }
            }
            _ => violations,
        }
    }

    /// Strip CWD prefix from a file path when `--relative` is set.
    fn rel_path(&self, path: &str) -> String {
        if self.cli.relative {
            let cwd_slash = format!("{}/", self.cwd.trim_end_matches('/'));
            if path.starts_with(&cwd_slash) {
                return path[cwd_slash.len()..].to_string();
            }
            if path.starts_with(&self.cwd) && path.len() > self.cwd.len() {
                let rest = &path[self.cwd.len()..];
                if rest.starts_with('/') {
                    return rest[1..].to_string();
                }
            }
        }
        path.to_string()
    }

    /// Write report content to `--reporter-output` file, or stdout if not set.
    fn emit(&self, content: &str) {
        if let Some(ref output_path) = self.cli.reporter_output {
            if let Some(parent) = std::path::Path::new(output_path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(mut f) = std::fs::File::create(output_path) {
                let _ = f.write_all(content.as_bytes());
                let _ = f.write_all(b"\n");
            }
        } else {
            println!("{}", content);
        }
    }

    fn report_plain(&self, violations: &[Violation]) -> i32 {
        let _total = violations.len();
        let limited = self.limited(violations);

        if limited.is_empty() {
            return 0;
        }

        let path_width = limited
            .iter()
            .map(|v| self.rel_path(&v.file).len())
            .max()
            .unwrap_or(20);

        let mut output = String::new();
        for v in limited {
            let path = self.rel_path(&v.file);
            let location = format!("{:>width$}:{}:{}", path, v.line, v.col, width = path_width);
            let rule = format!("({})", v.rule_id);
            let msg = &v.message;

            if self.cli.color {
                output.push_str(&format!(
                    "{} {} {}\n",
                    location.red(),
                    rule.dimmed(),
                    msg.yellow()
                ));
            } else {
                output.push_str(&format!("{} {} {}\n", location, rule, msg));
            }
        }

        // Summary
        if self.cli.format {
            eprintln!(
                "\n{}",
                "Lint has found errors that can be autocorrected using 'ktlint --format'".yellow()
            );
        }
        self.emit(&output);
        self.print_summary(violations); // summary always uses full count
        1
    }

    fn report_json(&self, violations: &[Violation]) -> i32 {
        let is_empty = violations.is_empty();
        let limited = self.limited(violations);

        let output: Vec<serde_json::Value> = limited
            .iter()
            .map(|v| {
                serde_json::json!({
                    "file": self.rel_path(&v.file),
                    "line": v.line,
                    "col": v.col,
                    "rule": v.rule_id,
                    "message": v.message,
                    "auto_fixable": v.auto_fixable,
                })
            })
            .collect();

        let json = serde_json::to_string_pretty(&output).unwrap_or_default();
        self.emit(&json);
        if is_empty {
            0
        } else {
            1
        }
    }

    fn report_sarif(&self, violations: &[Violation]) -> i32 {
        let is_empty = violations.is_empty();
        let limited = self.limited(violations);

        let results: Vec<serde_json::Value> = limited
            .iter()
            .map(|v| {
                serde_json::json!({
                    "ruleId": v.rule_id,
                    "message": { "text": v.message },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": { "uri": self.rel_path(&v.file) },
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

        self.emit(&serde_json::to_string_pretty(&sarif).unwrap_or_default());
        if is_empty {
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
        let is_empty = violations.is_empty();
        let limited = self.limited(violations);

        let mut html = String::from(
            "<!DOCTYPE html>\n<html><head><meta charset=\"UTF-8\">\
             <title>ktlint-rs Report</title>\n<style>\n\
             body{font-family:monospace;margin:20px;background:#0d1117;color:#c9d1d9}\n\
             h1{color:#58a6ff}.violation{margin:4px 0;padding:8px;border-left:3px solid #f85149;background:#161b22}\n\
             .file{color:#8b949e}.rule{color:#d2a8ff}.line{color:#ffa657}.msg{color:#f85149}\n\
             .summary{color:#7ee787}th,td{text-align:left;padding:4px 12px}\n\
             </style></head><body><h1>ktlint-rs Report</h1>\n",
        );
        if is_empty {
            html.push_str("<p>No violations found.</p>\n");
        } else {
            html.push_str(&format!("<p>{} violations found.</p>\n", violations.len()));
            // Summary table
            let mut counts: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for v in violations {
                *counts.entry(&v.rule_id).or_default() += 1;
            }
            let mut sorted: Vec<_> = counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
            html.push_str("<table class=\"summary\"><tr><th>Rule</th><th>Count</th></tr>\n");
            for (rule, count) in &sorted {
                html.push_str(&format!("<tr><td>{}</td><td>{}</td></tr>\n", rule, count));
            }
            html.push_str("</table>\n");
            // Violation list
            for v in limited {
                html.push_str(&format!(
                    "<div class=\"violation\">\
                     <span class=\"file\">{}</span>\
                     <span class=\"line\">:{}:{}</span>\
                     <span class=\"rule\">({})</span>\
                     <span class=\"msg\">{}</span></div>\n",
                    self.rel_path(&v.file),
                    v.line,
                    v.col,
                    v.rule_id,
                    v.message
                ));
            }
        }
        html.push_str("</body></html>\n");
        self.emit(&html);
        self.print_summary(violations);
        if is_empty {
            0
        } else {
            1
        }
    }

    fn report_checkstyle_xml(&self, violations: &[Violation]) -> i32 {
        let is_empty = violations.is_empty();
        let limited = self.limited(violations);

        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<checkstyle version=\"1.0\">\n");
        // Group by file
        let mut files: std::collections::HashMap<&str, Vec<&Violation>> =
            std::collections::HashMap::new();
        for v in limited {
            files.entry(&v.file).or_default().push(v);
        }
        let mut file_names: Vec<&&str> = files.keys().collect();
        file_names.sort();
        for file_name in file_names {
            xml.push_str(&format!(
                "<file name=\"{}\">\n",
                xml_attr_escape(&self.rel_path(file_name))
            ));
            for v in &files[file_name] {
                xml.push_str(&format!(
                    "<error line=\"{}\" column=\"{}\" severity=\"error\" \
                     message=\"{}\" source=\"{}\"/>\n",
                    v.line,
                    v.col,
                    xml_attr_escape(&v.message),
                    xml_attr_escape(&v.rule_id)
                ));
            }
            xml.push_str("</file>\n");
        }
        xml.push_str("</checkstyle>\n");
        self.emit(&xml);
        self.print_summary(violations);
        if is_empty {
            0
        } else {
            1
        }
    }

    fn report_markdown(&self, violations: &[Violation]) -> i32 {
        let is_empty = violations.is_empty();
        let limited = self.limited(violations);

        let mut md = String::from("# ktlint-rs Report\n\n");
        if is_empty {
            md.push_str("No violations found.\n");
        } else {
            md.push_str(&format!("**{} violations** found.\n\n", violations.len()));
            md.push_str("| Rule | Count |\n|------|-------|\n");
            let mut counts: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for v in violations {
                *counts.entry(&v.rule_id).or_default() += 1;
            }
            let mut sorted: Vec<_> = counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
            for (rule, count) in &sorted {
                md.push_str(&format!("| {} | {} |\n", rule, count));
            }
            md.push_str("\n## Violations\n\n");
            md.push_str("| File | Line | Rule | Message |\n|------|------|------|--------|\n");
            for v in limited {
                md.push_str(&format!(
                    "| {} | {}:{} | {} | {} |\n",
                    self.rel_path(&v.file),
                    v.line,
                    v.col,
                    v.rule_id,
                    v.message
                ));
            }
        }
        self.emit(&md);
        self.print_summary(violations);
        if is_empty {
            0
        } else {
            1
        }
    }
}

fn xml_attr_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::rules::Violation;

    fn make_violation(file: &str, rule: &str) -> Violation {
        Violation {
            file: file.to_string(),
            line: 1,
            col: 1,
            rule_id: rule.to_string(),
            message: format!("{} violation", rule),
            auto_fixable: true,
        }
    }

    fn make_cli(
        reporter: &str,
        limit: Option<usize>,
        reporter_output: Option<&str>,
        relative: bool,
    ) -> Cli {
        Cli {
            format: false,
            patterns_from_stdin: vec![],
            editorconfig: None,
            code_style: None,
            baseline: None,
            create_baseline: false,
            config: None,
            ruleset: "ktlint".to_string(),
            limit,
            relative,
            color: false,
            reporter: reporter.to_string(),
            reporter_output: reporter_output.map(|s| s.to_string()),
            log_level: None,
            patterns: vec![],
        }
    }

    // ── --limit ──

    #[test]
    fn t2_1_limit_truncates_json() {
        let cli = make_cli("json", Some(2), None, false);
        let r = DiagnosticReporter::new(&cli);
        let violations: Vec<Violation> = (0..5)
            .map(|i| make_violation(&format!("file{}.kt", i), "rule"))
            .collect();
        let limited = r.limited(&violations);
        assert_eq!(limited.len(), 2);
    }

    #[test]
    fn t2_2_limit_none_shows_all() {
        let cli = make_cli("json", None, None, false);
        let r = DiagnosticReporter::new(&cli);
        let violations: Vec<Violation> = (0..5)
            .map(|i| make_violation(&format!("file{}.kt", i), "rule"))
            .collect();
        assert_eq!(r.limited(&violations).len(), 5);
    }

    #[test]
    fn t2_3_limit_exceeds_total() {
        let cli = make_cli("json", Some(100), None, false);
        let r = DiagnosticReporter::new(&cli);
        let violations: Vec<Violation> = (0..5)
            .map(|i| make_violation(&format!("file{}.kt", i), "rule"))
            .collect();
        assert_eq!(r.limited(&violations).len(), 5);
    }

    // ── --reporter-output ──

    #[test]
    fn t2_4_reporter_output_writes_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = tmp.path().join("report.json");
        let cli = make_cli("json", None, Some(&out.to_string_lossy()), false);
        let r = DiagnosticReporter::new(&cli);
        let violations = vec![make_violation("test.kt", "rule-a")];
        let ret = r.report(&violations);
        assert_eq!(ret, 1);
        assert!(out.exists());
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("test.kt"));
        assert!(content.contains("rule-a"));
    }

    #[test]
    fn t2_5_reporter_output_none_uses_stdout() {
        // Just verify no panic with None
        let cli = make_cli("json", None, None, false);
        let r = DiagnosticReporter::new(&cli);
        let violations = vec![make_violation("test.kt", "rule-a")];
        r.report(&violations); // writes to stdout via println!
    }

    // ── --relative ──

    #[test]
    fn t2_7_relative_strips_cwd() {
        let mut cli = make_cli("json", None, None, true);
        let cwd = std::env::current_dir().unwrap();
        let r = DiagnosticReporter {
            cli: &cli,
            cwd: cwd.to_string_lossy().to_string(),
        };
        let path = format!("{}/src/Foo.kt", cwd.to_string_lossy());
        assert_eq!(r.rel_path(&path), "src/Foo.kt");
    }

    #[test]
    fn t2_8_relative_no_trailing_slash() {
        let mut cli = make_cli("json", None, None, true);
        let cwd = std::env::current_dir().unwrap();
        let r = DiagnosticReporter {
            cli: &cli,
            cwd: format!("{}/", cwd.to_string_lossy()),
        };
        let path = format!("{}/src/Foo.kt", cwd.to_string_lossy().trim_end_matches('/'));
        assert_eq!(r.rel_path(&path), "src/Foo.kt");
    }

    #[test]
    fn t2_9_relative_not_matching_keeps_absolute() {
        let mut cli = make_cli("json", None, None, true);
        let r = DiagnosticReporter {
            cli: &cli,
            cwd: "/project".to_string(),
        };
        assert_eq!(r.rel_path("/other/src/Foo.kt"), "/other/src/Foo.kt");
    }

    #[test]
    fn t2_14_limit_with_summary_total() {
        let cli = make_cli("plain", Some(2), None, false);
        let r = DiagnosticReporter::new(&cli);
        let violations: Vec<Violation> = (0..10)
            .map(|i| make_violation(&format!("file{}.kt", i), "rule"))
            .collect();
        // Limited should be 2, but summary uses full violations
        let limited = r.limited(&violations);
        assert_eq!(limited.len(), 2);
    }

    // ── All reporters smoke test ──

    #[test]
    fn t2_15_all_reporters_no_panic() {
        for reporter in &[
            "json",
            "sarif",
            "plain-summary",
            "plain",
            "html",
            "xml",
            "checkstyle",
            "markdown",
        ] {
            let cli = make_cli(reporter, Some(1), None, false);
            let r = DiagnosticReporter::new(&cli);
            let violations = vec![make_violation("test.kt", "r1")];
            r.report(&violations); // should not panic
        }
    }
}
