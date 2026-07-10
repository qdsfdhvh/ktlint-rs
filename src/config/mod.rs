//! ktlint configuration loaded from .editorconfig files and CLI flags.
//!
//! Supports:
//! - `.editorconfig` file cascade
//! - `ktlint_*` prefixed properties for rule control
//! - Code style profiles: android_studio, intellij_idea, ktlint_official
//! - CLI override for all config values

use crate::cli::Cli;
use std::collections::HashMap;
use std::path::PathBuf;

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

impl KtlintConfig {
    /// Load config from .editorconfig and CLI flags.
    pub fn load(cli: &Cli) -> anyhow::Result<Self> {
        let project_root = std::env::current_dir()?;
        let mut config = Self {
            project_root,
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
                _ => {}
            }
        }
    }

    /// Check whether a rule is enabled.
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        if let Some(rule_config) = self.rules.get(rule_id) {
            return rule_config.enabled;
        }
        if self.code_style.is_rule_disabled(rule_id) {
            return false;
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
}
