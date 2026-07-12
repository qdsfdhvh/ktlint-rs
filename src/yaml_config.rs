//! YAML configuration support — optional detekt-style config format.
//!
//! Format:
//! ```yaml
//! rules:
//!   indent:
//!     active: true
//!     indent_size: 4
//!   final-newline:
//!     active: false
//! ```

use crate::config::{IndentStyle, KtlintConfig, RuleConfig};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

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

fn default_active() -> bool {
    true
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
    if let Some(n) = yaml.indent_size {
        config.indent_size = n;
    }
    if let Some(ref s) = yaml.indent_style {
        config.indent_style = if s == "tab" {
            IndentStyle::Tab
        } else {
            IndentStyle::Space
        };
    }
    if let Some(n) = yaml.max_line_length {
        config.max_line_length = n;
    }
    if let Some(b) = yaml.insert_final_newline {
        config.insert_final_newline = b;
    }

    // Apply rule configs
    for (rule_name, rule_cfg) in &yaml.rules {
        let rule_id = format!("standard:{}", rule_name);
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
        config.rules.insert(
            rule_id,
            RuleConfig {
                enabled: rule_cfg.active,
                properties,
            },
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_yaml() {
        let yaml_str = r#"
rules:
  indent:
    active: true
    indent_size: 2
  final-newline:
    active: false
  function-naming:
    active: true
    properties:
      ignore_when_annotated_with: "Composable,Test"
code_style: android_studio
indent_size: 4
"#;
        let yaml: YamlConfig = serde_yaml::from_str(yaml_str).unwrap();
        assert_eq!(yaml.code_style, Some("android_studio".into()));
        assert_eq!(yaml.indent_size, Some(4));
        assert!(yaml.rules["indent"].active);
        assert!(!yaml.rules["final-newline"].active);
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
        assert!(!config.is_rule_enabled("standard:no-wildcard-imports")); // android_studio
    }
}
