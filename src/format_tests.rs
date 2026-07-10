#[cfg(test)]
mod format_tests {
    use crate::parser::KotlinParser;
    use crate::rules::{RuleEngine, Violation};
    use crate::config::KtlintConfig;
    use crate::formatter;
    use tempfile::NamedTempFile;
    use std::io::Write;

    fn lint_and_fix(source: &str, tmp: &tempfile::NamedTempFile) -> (Vec<Violation>, String) {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        let engine = RuleEngine::new(&KtlintConfig::default());
        let mut violations = engine.check("test.kt", &tree, source);
        // Fix the file path to point to the temp file
        for v in &mut violations {
            v.file = tmp.path().to_string_lossy().to_string();
        }
        formatter::auto_fix(&[tmp.path().to_path_buf()], &violations).unwrap();
        let after = std::fs::read_to_string(tmp.path()).unwrap();
        (violations, after)
    }

    #[test]
    fn format_idempotency() {
        let source = "class Foo{ fun bar( x:Int):String=\"\" }";
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(source.as_bytes()).unwrap();

        let (_, after1) = lint_and_fix(source, &f);
        assert_ne!(source, after1, "Format should change the file");

        // Round 2: lint → format on the already-formatted file
        let (v2, after2) = lint_and_fix(&after1, &f);
        // After format, most spacing violations should be gone
        let spacing_remain: Vec<_> = v2.iter().filter(|v| v.rule_id.contains("spacing") || v.rule_id.contains("curly")).collect();
        assert!(spacing_remain.len() <= 5, "Format should fix most spacing violations, got {}: {:?}", spacing_remain.len(), spacing_remain.iter().map(|v| &v.rule_id).collect::<Vec<_>>());

        // Should be idempotent (no further changes after second format)
        assert_eq!(after1.trim(), after2.trim(), "Format should be idempotent");
    }

    #[test]
    fn format_reduces_violations() {
        let source = "class Foo{\n    fun bar(x:Int,y:String):Boolean=x>0\n}   ";
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(source.as_bytes()).unwrap();

        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        let engine = RuleEngine::new(&KtlintConfig::default());
        let before = engine.check("test.kt", &tree, source).len();

        let mut violations = engine.check("test.kt", &tree, source);
        for v in &mut violations {
            v.file = f.path().to_string_lossy().to_string();
        }
        formatter::auto_fix(&[f.path().to_path_buf()], &violations).unwrap();

        let after_source = std::fs::read_to_string(f.path()).unwrap();
        let tree2 = parser.parse(&after_source);
        let after = engine.check("test.kt", &tree2, &after_source).len();

        assert!(after < before, "Format should reduce violations: {} → {}", before, after);
    }
}
