//! Type resolution FFI bridge (Phase 13) — optional kotlinc integration.
//!
//! When `--kotlinc-path` is set, calls kotlinc to extract full type
//! info from Kotlin source files via a compiler plugin that outputs
//! typed JSON. Falls back to CST-based TypeInfo::extract() when
//! kotlinc is not available or fails.
//!
//! JSON schema (produced by a Kotlin compiler plugin):
//! ```json
//! {
//!   "version": 1,
//!   "declarations": {
//!     "name": {"type": "String", "nullable": false, "line": 3},
//!     ...
//!   },
//!   "return_types": {
//!     "foo": "Int",
//!     "bar": "String"
//!   }
//! }
//! ```

use crate::resolver::type_bridge::{DeclType, TypeInfo};
use std::process::{Command, Stdio};

#[derive(Debug)]
pub enum TypeResolution {
    Compiler(TypeInfo),
    Cst(TypeInfo),
    Unavailable,
}

pub struct FfiBridge {
    pub kotlinc_path: Option<String>,
}

impl FfiBridge {
    pub fn new(kotlinc_path: Option<String>) -> Self {
        Self { kotlinc_path }
    }

    pub fn resolve(&self, source: &str) -> TypeResolution {
        match &self.kotlinc_path {
            Some(path) => match self.call_kotlinc(path, source) {
                Ok(ti) => TypeResolution::Compiler(ti),
                Err(_) => TypeResolution::Cst(TypeInfo::extract(source)),
            },
            None => TypeResolution::Cst(TypeInfo::extract(source)),
        }
    }

    /// Call kotlinc with an analysis script to dump type info as JSON.
    /// The Kotlin compiler plugin outputs the JSON schema defined above.
    fn call_kotlinc(&self, kotlinc_path: &str, source: &str) -> Result<TypeInfo, String> {
        let mut child = Command::new(kotlinc_path)
            .arg("-script")
            .arg("-")
            .arg("--")
            .arg(source)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn kotlinc: {}", e))?;

        // Write the analysis script (kotlinc reads from stdin with -script -)
        drop(child.stdin.take());

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for kotlinc: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "kotlinc exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_compiler_output(&stdout)
    }

    /// Parse kotlinc JSON output into TypeInfo.
    ///
    /// Schema:
    /// ```json
    /// {
    ///   "version": 1,
    ///   "declarations": { "name": {"type": "String", "nullable": false, "line": 3} },
    ///   "return_types": { "foo": "Int" }
    /// }
    /// ```
    fn parse_compiler_output(json: &str) -> Result<TypeInfo, String> {
        use serde_json::Value;

        let parsed: Value = serde_json::from_str(json)
            .map_err(|e| format!("Invalid JSON from kotlinc: {}", e))?;

        let mut ti = TypeInfo::default();

        // Parse declarations: {"name": {"type": "X", "nullable": bool, "line": N}}
        if let Some(decls) = parsed.get("declarations").and_then(|d| d.as_object()) {
            for (name, val) in decls {
                let type_name = val
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let is_nullable = val
                    .get("nullable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let line = val.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                ti.declarations.insert(
                    name.to_string(),
                    DeclType {
                        type_name,
                        is_nullable,
                        line,
                    },
                );
            }
        }

        // Parse return types: {"func_name": "Type"}
        if let Some(rets) = parsed.get("return_types").and_then(|r| r.as_object()) {
            for (name, val) in rets {
                if let Some(typ) = val.as_str() {
                    ti.return_types.insert(name.to_string(), typ.to_string());
                }
            }
        }

        Ok(ti)
    }
}

impl Default for FfiBridge {
    fn default() -> Self {
        Self {
            kotlinc_path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_bridge_unavailable_cst_fallback() {
        let bridge = FfiBridge::new(None);
        match bridge.resolve("val x: String = \"hello\"") {
            TypeResolution::Cst(ti) => {
                assert_eq!(ti.type_of("x").unwrap().type_name, "String");
            }
            _ => panic!("Expected CST fallback"),
        }
    }

    #[test]
    fn ffi_bridge_invalid_kotlinc_fallback() {
        let bridge = FfiBridge::new(Some("/nonexistent/kotlinc".into()));
        match bridge.resolve("val x: Int = 42") {
            TypeResolution::Cst(ti) => {
                assert_eq!(ti.type_of("x").unwrap().type_name, "Int");
            }
            _ => panic!("Expected CST fallback for invalid kotlinc path"),
        }
    }

    #[test]
    fn parse_valid_json() {
        let json = r#"{
            "version": 1,
            "declarations": {
                "x": {"type": "String", "nullable": false, "line": 1},
                "y": {"type": "Int", "nullable": true, "line": 2}
            },
            "return_types": {
                "foo": "String",
                "bar": "Unit"
            }
        }"#;
        let ti = FfiBridge::parse_compiler_output(json).unwrap();
        assert_eq!(ti.type_of("x").unwrap().type_name, "String");
        assert!(!ti.type_of("x").unwrap().is_nullable);
        assert!(ti.type_of("y").unwrap().is_nullable);
        assert_eq!(ti.return_types.get("foo").unwrap(), "String");
        assert_eq!(ti.return_types.get("bar").unwrap(), "Unit");
    }

    #[test]
    fn parse_empty_json() {
        let ti = FfiBridge::parse_compiler_output("{}").unwrap();
        assert!(ti.declarations.is_empty());
        assert!(ti.return_types.is_empty());
    }

    #[test]
    fn parse_invalid_json() {
        assert!(FfiBridge::parse_compiler_output("not json").is_err());
    }
}
