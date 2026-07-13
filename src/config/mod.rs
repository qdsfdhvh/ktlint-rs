//! ktlint configuration loaded from .editorconfig files and CLI flags.
//!
//! Supports:
//! - `.editorconfig` file cascade
//! - `ktlint_*` prefixed properties for rule control
//! - Code style profiles: android_studio, intellij_idea, ktlint_official
//! - CLI override for all config values

use crate::cli::Cli;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// ktlint configuration.
#[derive(Debug, Clone)]
pub struct KtlintConfig {
    pub editorconfig_path: Option<PathBuf>,
    pub code_style: CodeStyle,
    pub baseline: Option<PathBuf>,
    pub rules: HashMap<String, RuleConfig>,
    pub experimental: bool,
    pub project_root: PathBuf,
    pub indent_size: usize,
    pub indent_style: IndentStyle,
    pub max_line_length: usize,
    pub insert_final_newline: bool,
    pub trim_trailing_whitespace: bool,
    /// Rule set filter (ktlint-only, detekt-only, or both)
    pub rule_set: RuleSet,
    /// Category-level overrides from YAML (e.g., "detekt:complexity" → RuleConfig)
    pub category_overrides: HashMap<String, RuleConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSet {
    KtlintOnly,
    DetektOnly,
    Both,
}

impl RuleSet {
    pub fn from_str(s: &str) -> Self {
        let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
        let has_ktlint = parts.iter().any(|p| *p == "ktlint");
        let has_detekt = parts.iter().any(|p| *p == "detekt");
        match (has_ktlint, has_detekt) {
            (true, true) => RuleSet::Both,
            (false, true) => RuleSet::DetektOnly,
            _ => RuleSet::KtlintOnly,
        }
    }
}
impl Default for KtlintConfig {
    fn default() -> Self {
        Self {
            editorconfig_path: None,
            code_style: CodeStyle::default(),
            baseline: None,
            rules: HashMap::new(),
            experimental: false,
            project_root: PathBuf::from("."),
            indent_size: 4,
            indent_style: IndentStyle::Space,
            max_line_length: 0,
            insert_final_newline: true,
            trim_trailing_whitespace: true,
            rule_set: RuleSet::KtlintOnly,
            category_overrides: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CodeStyle {
    #[default]
    KtlintOfficial,
    AndroidStudio,
    IntelliJIdea,
}

impl CodeStyle {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "android_studio" | "android" => Self::AndroidStudio,
            "intellij_idea" | "intellij" => Self::IntelliJIdea,
            _ => Self::KtlintOfficial,
        }
    }

    /// Returns whether a rule is disabled by this code style profile.
    pub fn is_rule_disabled(&self, rule_id: &str) -> bool {
        match self {
            Self::AndroidStudio => matches!(
                rule_id,
                "standard:final-newline"
                    | "standard:no-wildcard-imports"
                    | "standard:import-ordering"
                    | "standard:trailing-comma"
                    | "standard:no-unused-imports"
                    | "standard:multiline-expression-wrapping"
                    | "standard:no-empty-first-line-in-class-body"
                    | "standard:argument-list-wrapping"
                    | "standard:no-consecutive-comments"
                    | "standard:blank-line-between-when-conditions"
                    | "standard:when-entry-bracing"
                    | "standard:no-blank-line-before-rbrace"
            ),
            Self::IntelliJIdea => matches!(
                rule_id,
                "standard:no-wildcard-imports"
                    | "standard:import-ordering"
                    | "standard:trailing-comma"
            ),
            Self::KtlintOfficial => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IndentStyle {
    #[default]
    Space,
    Tab,
}

#[derive(Debug, Clone, Default)]
pub struct RuleConfig {
    pub enabled: bool,
    pub properties: HashMap<String, String>,
}

/// Walk up from a file path to find the nearest `.editorconfig` file.
fn find_editorconfig(file_path: &Path) -> Option<PathBuf> {
    let mut current = if file_path.is_dir() {
        file_path.to_path_buf()
    } else {
        file_path.parent()?.to_path_buf()
    };
    loop {
        let candidate = current.join(".editorconfig");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Parse ktlint-specific properties from a raw `.editorconfig` file.
/// `file_path` is the Kotlin file being linted — used to match the right section.
fn parse_ktlint_properties(ec_path: &Path, file_path: &Path) -> HashMap<String, String> {
    let mut props = HashMap::new();
    let content = match std::fs::read_to_string(ec_path) {
        Ok(c) => c,
        Err(_) => return props,
    };

    // Determine file extension for section matching.
    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len() - 1];
            // Match: * or *.kt or *.{kt,kts} or {*.kt,*.kts}
            in_section = section == "*"
                || section == &format!("*.{}", ext)
                || section.contains(&format!("{{{}}}", ext))
                || section.contains(&format!(".{}, ", ext))
                || section.contains(&format!("*.{}", ext));
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if key.starts_with("ktlint_") || key.starts_with("ij_kotlin_") {
                props.insert(key.to_string(), value.to_string());
            }
        }
    }
    props
}

impl KtlintConfig {
    /// Load config from .editorconfig and CLI flags.
    pub fn load(cli: &Cli) -> anyhow::Result<Self> {
        let project_root = std::env::current_dir()?;
        let mut config = Self {
            project_root,
            rule_set: RuleSet::from_str(&cli.ruleset),
            ..Default::default()
        };

        let anchor_path = if let Some(first) = cli.patterns.first() {
            PathBuf::from(first)
        } else {
            config.project_root.clone()
        };

        let anchor_path = if anchor_path.is_absolute() {
            anchor_path
        } else {
            config.project_root.join(&anchor_path)
        };

        // Convert directory→file path (editorconfig crate needs a file)
        let ec_lookup_path = if anchor_path.is_dir() {
            anchor_path.join("dummy.kt")
        } else {
            anchor_path.clone()
        };

        // Load .editorconfig → convert OrderMap to HashMap to avoid version conflicts
        if let Ok(ec_map) = editorconfig::get_config(&ec_lookup_path) {
            let map: HashMap<String, String> = ec_map.into_iter().collect();
            config.apply_editorconfig(&map);
            // Also parse ktlint-specific properties not returned by editorconfig crate.
            if let Some(ec_file) = find_editorconfig(&ec_lookup_path) {
                let ktlint_props = parse_ktlint_properties(&ec_file, &ec_lookup_path);
                config.apply_editorconfig(&ktlint_props);
            }
            if let Some(ref ec_path) = cli.editorconfig {
                if let Ok(named) =
                    editorconfig::get_config_conffile(&ec_lookup_path, ec_path.as_str())
                {
                    let named_map: HashMap<String, String> = named.into_iter().collect();
                    config.apply_editorconfig(&named_map);
                }
            }
        }

        // CLI overrides always win
        if let Some(ref style) = cli.code_style {
            config.code_style = CodeStyle::from_str(style);
        }
        if let Some(ref b) = cli.baseline {
            config.baseline = Some(PathBuf::from(b));
        }

        Ok(config)
    }

    /// Load config for a specific file path (multi-project support).
    pub fn load_for_file(file_path: &Path) -> anyhow::Result<Self> {
        let mut config = Self::default();
        // Ensure absolute path so editorconfig::get_config resolves correctly.
        let abs_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir()?.join(file_path)
        };

        // Standard EditorConfig properties (indent_size, indent_style, etc.)
        if let Ok(ec_map) = editorconfig::get_config(&abs_path) {
            let map: std::collections::HashMap<String, String> = ec_map.into_iter().collect();
            config.apply_editorconfig(&map);
        }

        // ktlint-specific properties (code_style, rule enable/disable, etc.)
        // Not returned by the editorconfig crate — we parse them directly.
        if let Some(ec_file) = find_editorconfig(&abs_path) {
            let ktlint_props = parse_ktlint_properties(&ec_file, &abs_path);
            config.apply_editorconfig(&ktlint_props);
        }

        config.project_root = file_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        Ok(config)
    }

    fn apply_editorconfig(&mut self, map: &HashMap<String, String>) {
        for (key, value) in map.iter() {
            let v: &str = value;
            match key.as_str() {
                "ktlint_code_style" => self.code_style = CodeStyle::from_str(v),
                "ktlint_experimental" if v == "enabled" => self.experimental = true,
                "indent_size" if v != "tab" => {
                    self.indent_size = v.parse().unwrap_or(4);
                }
                "indent_style" => {
                    self.indent_style = if v == "tab" {
                        IndentStyle::Tab
                    } else {
                        IndentStyle::Space
                    };
                }
                "max_line_length" => {
                    if let Ok(n) = v.parse() {
                        self.max_line_length = n;
                    }
                }
                "insert_final_newline" => self.insert_final_newline = v != "false",
                "trim_trailing_whitespace" => self.trim_trailing_whitespace = v != "false",
                key if key.starts_with("ktlint_standard_") => {
                    let rule_id = format!(
                        "standard:{}",
                        key.trim_start_matches("ktlint_standard_").replace('_', "-")
                    );
                    let enabled = v != "disabled" && v != "false";
                    self.rules
                        .entry(rule_id)
                        .or_insert_with(|| RuleConfig {
                            enabled,
                            properties: HashMap::new(),
                        })
                        .enabled = enabled;
                }
                key if key.starts_with("ij_kotlin_") => {
                    // IntelliJ properties: ij_kotlin_allow_trailing_comma, etc.
                    self.rules
                        .entry("ij_kotlin_properties".to_string())
                        .or_insert_with(|| RuleConfig {
                            enabled: true,
                            properties: HashMap::new(),
                        })
                        .properties
                        .insert(key.to_string(), v.to_string());
                }
                key if key.contains("_ignore_when_annotated_with") => {
                    // Rule-specific: ktlint_function_naming_ignore_when_annotated_with=Composable
                    self.rules
                        .entry(key.to_string())
                        .or_insert_with(|| RuleConfig {
                            enabled: true,
                            properties: HashMap::new(),
                        })
                        .properties
                        .insert("annotated_with".to_string(), v.to_string());
                }
                _ => {}
            }
        }
    }

    /// Check whether a rule is enabled.
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        // 1. Code style disables certain rules
        if self.code_style.is_rule_disabled(rule_id) {
            return false;
        }
        // 2. Category-level overrides (e.g., "detekt:complexity" → disabled)
        for (prefix, rc) in &self.category_overrides {
            if rule_id.starts_with(prefix) && !rc.enabled {
                return false;
            }
        }
        // 3. Per-rule enable/disable
        if let Some(rule_config) = self.rules.get(rule_id) {
            return rule_config.enabled;
        }
        true
    }

    pub fn indent_string(&self) -> String {
        match self.indent_style {
            IndentStyle::Space => " ".repeat(self.indent_size),
            IndentStyle::Tab => "\t".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_all_rules_enabled() {
        let config = KtlintConfig::default();
        assert!(config.is_rule_enabled("standard:curly-spacing"));
        assert!(config.is_rule_enabled("standard:no-wildcard-imports"));
    }

    #[test]
    fn android_disables_rules() {
        let config = KtlintConfig {
            code_style: CodeStyle::AndroidStudio,
            ..Default::default()
        };
        assert!(!config.is_rule_enabled("standard:final-newline"));
        assert!(config.is_rule_enabled("standard:curly-spacing"));
    }

    #[test]
    fn intellij_disables_rules() {
        let config = KtlintConfig {
            code_style: CodeStyle::IntelliJIdea,
            ..Default::default()
        };
        assert!(!config.is_rule_enabled("standard:no-wildcard-imports"));
        assert!(config.is_rule_enabled("standard:final-newline"));
    }

    #[test]
    fn per_rule_override() {
        let mut config = KtlintConfig::default();
        config.rules.insert(
            "standard:curly-spacing".to_string(),
            RuleConfig {
                enabled: false,
                properties: Default::default(),
            },
        );
        assert!(!config.is_rule_enabled("standard:curly-spacing"));
    }

    #[test]
    fn nowinandroid_code_style_is_android_studio() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/nowinandroid/src/main/kotlin/com/google/samples/apps/nowinandroid/MainActivity.kt");
        if !path.exists() {
            return;
        }
        let config = KtlintConfig::load_for_file(&path).unwrap();
        assert_eq!(
            config.code_style,
            CodeStyle::AndroidStudio,
            "code_style should be android_studio"
        );
        assert!(!config.is_rule_enabled("standard:multiline-expression-wrapping"));
        assert!(config.is_rule_enabled("standard:colon-spacing"));
    }

    #[test]
    fn indent_string_4_space_default() {
        let config = KtlintConfig::default();
        assert_eq!(config.indent_string(), "    ");
    }

    #[test]
    fn indent_string_tab() {
        let mut config = KtlintConfig::default();
        config.indent_style = IndentStyle::Tab;
        assert_eq!(config.indent_string(), "\t");
    }
}
