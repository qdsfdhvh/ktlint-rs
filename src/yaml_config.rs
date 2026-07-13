//! YAML configuration support — detekt-style config format.
//!
//! Supports:
//! - Namespace inference: short names → auto-prefix (standard:/detekt:)
//! - Category-level batch switches
//! - Properties per rule

use crate::config::{IndentStyle, KtlintConfig, RuleConfig};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

// Known detekt rule short names → full ID prefix
const DETEKT_CATEGORIES: &[&str] = &[
    "empty-blocks", "complexity", "style", "naming", "comments",
    "exceptions", "potential-bugs", "performance", "coroutines", "libraries",
];

/// Top-level YAML config structure.
#[derive(Debug, Deserialize, Default)]
pub struct YamlConfig {
    #[serde(default)]
    pub rules: HashMap<String, YamlRuleConfig>,
    #[serde(default)]
    pub code_style: Option<String>,
    #[serde(default)]
    pub indent_size: Option<usize>,
    #[serde(default)]
    pub indent_style: Option<String>,
    #[serde(default)]
    pub max_line_length: Option<usize>,
    #[serde(default)]
    pub insert_final_newline: Option<bool>,
}

/// Per-rule YAML config.
#[derive(Debug, Deserialize, Default)]
pub struct YamlRuleConfig {
    #[serde(default = "default_active")]
    pub active: bool,
    #[serde(default)]
    pub properties: HashMap<String, serde_yaml::Value>,
}

fn default_active() -> bool { true }

/// Resolve a rule name from YAML to its full ID.
/// - Contains `:` → full ID, use as-is
/// - Matches known detekt short name → prefix with detekt:<category>:
/// - Otherwise → prefix with standard:
fn resolve_rule_id(name: &str) -> String {
    if name.contains(':') {
        return name.to_string();
    }
    // Check if it's a known detekt rule: "<category>:<rule>" or "<rule>"
    // we look for known detekt categories
    for cat in DETEKT_CATEGORIES {
        if name.starts_with(&format!("{}/", cat)) {
            // Not standard naming
            continue;
        }
    }
    // Check if name is a known detekt short name
    // For now, heuristic: if it looks like a detekt rule name (PascalCase, contains certain patterns)
    // then prefix with detekt: otherwise standard:
    let looks_like_detekt = name.chars().next().map_or(false, |c| c.is_uppercase())
        && (name.contains("Method") || name.contains("Class") || name.contains("Function")
            || name.contains("Block") || name.contains("Condition") || name.contains("Complex")
            || name.contains("Naming") || name.contains("Comment"));
    if looks_like_detekt {
        // Try to infer category from name suffix
        let category = guess_detekt_category(name);
        format!("detekt:{}:{}", category, name)
    } else {
        format!("standard:{}", name)
    }
}

fn guess_detekt_category(name: &str) -> &str {
    if name.contains("Method") || name.contains("Class") || name.contains("Parameter")
        || name.contains("Condition") || name.contains("Complex") || name.contains("Depth")
        || name.contains("Block") || name.contains("Function") && name.contains("Many")
    {
        "complexity"
    } else if name.contains("Empty") && name.contains("Block") {
        "empty-blocks"
    } else if name.contains("Naming") {
        "naming"
    } else if name.contains("Comment") {
        "comments"
    } else {
        "style"
    }
}

/// Load YAML config and apply it to a KtlintConfig.
pub fn load_and_apply(config: &mut KtlintConfig, yaml_path: &Path) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(yaml_path)?;
    let yaml: YamlConfig = serde_yaml::from_str(&content)?;

    // Apply code style
    if let Some(ref style) = yaml.code_style {
        config.code_style = crate::config::CodeStyle::from_str(style);
    }

    // Apply global settings
    if let Some(n) = yaml.indent_size { config.indent_size = n; }
    if let Some(ref s) = yaml.indent_style {
        config.indent_style = if s == "tab" { IndentStyle::Tab } else { IndentStyle::Space };
    }
    if let Some(n) = yaml.max_line_length { config.max_line_length = n; }
    if let Some(b) = yaml.insert_final_newline { config.insert_final_newline = b; }

    // Apply rule configs with namespace inference + category-level support
    for (rule_name, rule_cfg) in &yaml.rules {
        let rule_id = resolve_rule_id(rule_name);
        let mut properties = HashMap::new();
        for (k, v) in &rule_cfg.properties {
            let val_str = match v {
                serde_yaml::Value::String(s) => s.clone(),
                serde_yaml::Value::Number(n) => n.to_string(),
                serde_yaml::Value::Bool(b) => b.to_string(),
                _ => continue,
            };
            properties.insert(k.clone(), val_str);
        }

        let rc = RuleConfig { enabled: rule_cfg.active, properties };

        // Category-level batch switch: if rule_name is "detekt:category" (contains : but no second : → category prefix)
        if rule_id.contains(':') && !rule_id.matches(':').count() >= 2 {
            // This is "detekt:category" format → apply to all rules in this category
            apply_category(config, &rule_id, &rc);
        } else {
            config.rules.insert(rule_id, rc);
        }
    }

    Ok(())
}

/// Apply a RuleConfig to all rules whose ID starts with the given prefix.
fn apply_category(config: &mut KtlintConfig, prefix: &str, rc: &RuleConfig) {
    // We need to know ALL possible rule IDs. Scan the registry.
    // For now, store a wildcard entry that is_rule_enabled can check.
    config.category_overrides.insert(prefix.to_string(), rc.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_standard() {
        assert_eq!(resolve_rule_id("final-newline"), "standard:final-newline");
        assert_eq!(resolve_rule_id("indent"), "standard:indent");
    }

    #[test]
    fn resolve_full_id() {
        assert_eq!(
            resolve_rule_id("detekt:empty-blocks:EmptyFunctionBlock"),
            "detekt:empty-blocks:EmptyFunctionBlock"
        );
    }

    #[test]
    fn resolve_detekt_short() {
        assert_eq!(
            resolve_rule_id("LongMethod"),
            "detekt:complexity:LongMethod"
        );
    }

    #[test]
    fn parse_yaml() {
        let yaml_str = r#"
rules:
  indent:
    active: true
    indent_size: 2
  final-newline:
    active: false
  detekt:empty-blocks:EmptyFunctionBlock:
    active: false
code_style: android_studio
indent_size: 4
"#;
        let yaml: YamlConfig = serde_yaml::from_str(yaml_str).unwrap();
        assert_eq!(yaml.code_style, Some("android_studio".into()));
        assert_eq!(yaml.indent_size, Some(4));
        assert!(yaml.rules["indent"].active);
        assert!(!yaml.rules["final-newline"].active);
        assert!(!yaml.rules["detekt:empty-blocks:EmptyFunctionBlock"].active);
    }

    #[test]
    fn apply_to_config() {
        let yaml_str = r#"
rules:
  final-newline:
    active: false
code_style: android_studio
indent_size: 2
"#;
        let tmp = std::env::temp_dir().join("ktlint_test_config.yml");
        std::fs::write(&tmp, yaml_str).unwrap();
        let mut config = KtlintConfig::default();
        load_and_apply(&mut config, &tmp).unwrap();
        std::fs::remove_file(&tmp).ok();
        assert!(!config.is_rule_enabled("standard:final-newline"));
        assert_eq!(config.indent_size, 2);
    }
}
