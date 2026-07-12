//! Baseline support — filter out known violations from previous runs.
//!
//! Format compatible with JVM ktlint baseline.xml:
//! ```xml
//! <?xml version="1.0" encoding="UTF-8"?>
//! <baseline version="1.0">
//!     <file name="path/to/file.kt">
//!         <error line="42" column="1" source="standard:rule-id" />
//!     </file>
//! </baseline>
//! ```

use crate::rules::Violation;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Baseline {
    /// (file, line, column, rule_id) -> present
    entries: HashSet<(String, usize, usize, String)>,
}

impl Baseline {
    /// Load a baseline from an XML file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let mut entries = HashSet::new();

        let mut current_file = String::new();
        for line in content.lines() {
            let trimmed = line.trim();

            // Extract file name
            if let Some(rest) = trimmed.strip_prefix(r#"<file name=""#) {
                if let Some(end) = rest.rfind(r#"""#) {
                    current_file = rest[..end].to_string();
                }
            }

            // Extract error entry
            if let Some(rest) = trimmed.strip_prefix(r#"<error "#) {
                let mut line_num: usize = 0;
                let mut col_num: usize = 0;
                let mut source = String::new();

                for part in rest.split_whitespace() {
                    if let Some(v) = part.strip_prefix(r#"line=""#) {
                        line_num = v.trim_end_matches('"').parse().unwrap_or(0);
                    } else if let Some(v) = part.strip_prefix(r#"column=""#) {
                        col_num = v.trim_end_matches('"').parse().unwrap_or(0);
                    } else if let Some(v) = part.strip_prefix(r#"source=""#) {
                        source = v.trim_end_matches('"').to_string();
                    }
                }

                if !current_file.is_empty() && line_num > 0 {
                    entries.insert((current_file.clone(), line_num, col_num, source));
                }
            }
        }

        Ok(Self { entries })
    }

    /// Check if a violation is in the baseline.
    pub fn contains(&self, file: &str, line: usize, col: usize, rule_id: &str) -> bool {
        // Try exact match first
        if self.entries.contains(&(file.to_string(), line, col, rule_id.to_string())) {
            return true;
        }
        // Try matching without column (some baselines use col=0)
        if col > 0 && self.entries.contains(&(file.to_string(), line, 0, rule_id.to_string())) {
            return true;
        }
        false
    }

    /// Filter out violations covered by the baseline.
    pub fn filter(&self, violations: Vec<Violation>) -> Vec<Violation> {
        violations
            .into_iter()
            .filter(|v| !self.contains(&v.file, v.line, v.col, &v.rule_id))
            .collect()
    }

    /// Generate a baseline XML from current violations (for --create-baseline).
    pub fn generate(violations: &[Violation]) -> String {
        let mut files: HashMap<String, Vec<&Violation>> = HashMap::new();
        for v in violations {
            files.entry(v.file.clone()).or_default().push(v);
        }

        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');
        xml.push_str(r#"<baseline version="1.0">"#);
        xml.push('\n');

        let mut file_names: Vec<&String> = files.keys().collect();
        file_names.sort();

        for file_name in file_names {
            let mut entries: Vec<&Violation> = files[file_name].clone();
            entries.sort_by_key(|v| (v.line, v.col));
            entries.dedup_by_key(|v| (v.line, v.col, &v.rule_id));

            xml.push_str(&format!(r#"    <file name="{}">"#, xml_escape(file_name)));
            xml.push('\n');
            for v in &entries {
                xml.push_str(&format!(
                    r#"        <error line="{}" column="{}" source="{}" />"#,
                    v.line,
                    v.col,
                    xml_escape(&v.rule_id)
                ));
                xml.push('\n');
            }
            xml.push_str(r#"    </file>"#);
            xml.push('\n');
        }
        xml.push_str(r#"</baseline>"#);
        xml.push('\n');
        xml
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_and_filter() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<baseline version="1.0">
    <file name="test.kt">
        <error line="10" column="1" source="standard:indent" />
        <error line="20" column="5" source="standard:max-line-length" />
    </file>
    <file name="other.kt">
        <error line="5" column="0" source="standard:no-wildcard-imports" />
    </file>
</baseline>
"#;
        let tmp = std::env::temp_dir().join("ktlint_test_baseline.xml");
        std::fs::write(&tmp, xml).unwrap();
        let baseline = Baseline::load(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert!(baseline.contains("test.kt", 10, 1, "standard:indent"));
        assert!(baseline.contains("test.kt", 20, 5, "standard:max-line-length"));
        assert!(baseline.contains("other.kt", 5, 0, "standard:no-wildcard-imports"));
        assert!(!baseline.contains("test.kt", 99, 1, "standard:indent"));
    }

    #[test]
    fn filter_violations() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<baseline version="1.0">
    <file name="test.kt">
        <error line="10" column="1" source="standard:indent" />
    </file>
</baseline>
"#;
        let tmp = std::env::temp_dir().join("ktlint_test_baseline2.xml");
        std::fs::write(&tmp, xml).unwrap();
        let baseline = Baseline::load(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        let violations = vec![
            Violation {
                file: "test.kt".into(),
                line: 10,
                col: 1,
                rule_id: "standard:indent".into(),
                message: "bad indent".into(),
                auto_fixable: true,
            },
            Violation {
                file: "test.kt".into(),
                line: 20,
                col: 1,
                rule_id: "standard:indent".into(),
                message: "bad indent".into(),
                auto_fixable: true,
            },
        ];

        let filtered = baseline.filter(violations);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].line, 20);
    }

    #[test]
    fn generate_baseline() {
        let violations = vec![
            Violation {
                file: "test.kt".into(),
                line: 10,
                col: 1,
                rule_id: "standard:indent".into(),
                message: "bad indent".into(),
                auto_fixable: true,
            },
        ];
        let xml = Baseline::generate(&violations);
        assert!(xml.contains("test.kt"));
        assert!(xml.contains(r#"line="10""#));
        assert!(xml.contains("standard:indent"));
    }
}
