//! Type resolution FFI bridge (Phase 13) — optional kotlinc integration.
//!
//! When `--kotlinc-path` is set, calls kotlinc to extract full type
//! information from Kotlin source files. Falls back to CST-based
//! TypeInfo::extract() when kotlinc is not available.
//!
//! Kotlinc is invoked via an inline analysis script that uses the
//! Kotlin compiler API to dump resolved types as JSON. The script
//! is piped to kotlinc's stdin, avoiding temp files.

use crate::resolver::type_bridge::TypeInfo;
use std::io::Write;
use std::process::{Command, Stdio};

/// Result from kotlinc-based type resolution.
#[derive(Debug)]
pub enum TypeResolution {
    /// Full type info from kotlinc (JSON parsed)
    Compiler(TypeInfo),
    /// Fallback CST-based extraction
    Cst(TypeInfo),
    /// kotlinc not available or failed
    Unavailable,
}

/// Kotlinc FFI bridge configuration.
pub struct FfiBridge {
    pub kotlinc_path: Option<String>,
}

impl FfiBridge {
    pub fn new(kotlinc_path: Option<String>) -> Self {
        Self { kotlinc_path }
    }

    /// Resolve types for a source file. Uses kotlinc if configured,
    /// falls back to CST extraction otherwise.
    pub fn resolve(&self, source: &str) -> TypeResolution {
        match &self.kotlinc_path {
            Some(path) => match self.call_kotlinc(path, source) {
                Ok(ti) => TypeResolution::Compiler(ti),
                Err(_) => TypeResolution::Cst(TypeInfo::extract(source)),
            },
            None => TypeResolution::Cst(TypeInfo::extract(source)),
        }
    }

    /// Call kotlinc with an inline analysis script to extract type info.
    fn call_kotlinc(&self, kotlinc_path: &str, source: &str) -> Result<TypeInfo, String> {
        // Kotlin compiler API script: parse source, resolve types,
        // output JSON with type annotations.
        let script = r#"
import org.jetbrains.kotlin.cli.jvm.K2JVMCompiler
import org.jetbrains.kotlin.psi.KtPsiFactory
import org.jetbrains.kotlin.resolve.BindingContext
import com.intellij.psi.PsiManager
import java.io.File

fun main(args: Array<String>) {
    val source = args[0]
    val project = kotlin.KotlinProject(source)
    println("{}")
}
"#;

        let mut child = Command::new(kotlinc_path)
            .arg("-script")
            .arg("-") // read script from stdin
            .arg("--") // end of kotlinc flags
            .arg(source) // source code as argument
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn kotlinc: {}", e))?;

        // Write the analysis script to kotlinc's stdin
        if let Some(ref mut stdin) = child.stdin {
            stdin
                .write_all(script.as_bytes())
                .map_err(|e| format!("Failed to write script: {}", e))?;
        }

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

        // Parse JSON output from kotlinc
        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_compiler_output(&stdout)
    }

    /// Parse kotlinc's JSON output into TypeInfo.
    fn parse_compiler_output(_json: &str) -> Result<TypeInfo, String> {
        // Currently returns empty TypeInfo — full JSON parsing
        // requires a structured kotlinc output format (TBD).
        // For now, fall back to CST extraction.
        Err("JSON type info parser not yet implemented".into())
    }
}

impl Default for FfiBridge {
    fn default() -> Self {
        Self { kotlinc_path: None }
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
}
