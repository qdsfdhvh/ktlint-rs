#[cfg(test)]
mod format_tests {
    use crate::config::KtlintConfig;
    use crate::formatter;
    use crate::parser::KotlinParser;
    use crate::rules::{RuleEngine, Violation};
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn lint_and_fix(source: &str, tmp: &tempfile::NamedTempFile) -> (Vec<Violation>, String) {
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        let engine = RuleEngine::new(&KtlintConfig::default());
        let mut violations = engine.check("test.kt", &tree, source);
        // Fix the file path to point to the temp file
        for v in &mut violations {
            v.file = tmp.path().to_string_lossy().to_string();
        }
        formatter::auto_fix(
            &[tmp.path().to_path_buf()],
            &violations,
            4,
            true,
            &Default::default(),
            // The spotless-parity goldens are generated with the reference
            // android_studio editorconfig (see fixtures README) — the
            // ktlint-official-only rules (multiline-expression-wrapping
            // etc.) must stay off to match them byte-for-byte.
            crate::config::CodeStyle::AndroidStudio,
            120,
        )
        .unwrap();
        let after = std::fs::read_to_string(tmp.path()).unwrap();
        (violations, after)
    }

    #[test]
    fn format_idempotency() {
        // Must parse cleanly: the formatter deliberately skips spacing edits on
        // files tree-sitter-kotlin-sg can't parse (safety over completeness), so a
        // grammar-breaking snippet would no-op and defeat this test's intent.
        let source = "class Foo {\n    val x:Int=1\n    fun bar(a:String) {}\n}\n";
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(source.as_bytes()).unwrap();

        let (_, after1) = lint_and_fix(source, &f);
        assert_ne!(source, after1, "Format should change the file");

        // Round 2: lint → format on the already-formatted file
        let (v2, after2) = lint_and_fix(&after1, &f);
        // After format, most spacing violations should be gone
        let spacing_remain: Vec<_> = v2
            .iter()
            .filter(|v| v.rule_id.contains("spacing") || v.rule_id.contains("curly"))
            .collect();
        assert!(
            spacing_remain.len() <= 5,
            "Format should fix most spacing violations, got {}: {:?}",
            spacing_remain.len(),
            spacing_remain
                .iter()
                .map(|v| &v.rule_id)
                .collect::<Vec<_>>()
        );

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
        formatter::auto_fix(
            &[f.path().to_path_buf()],
            &violations,
            4,
            true,
            &Default::default(),
            crate::config::CodeStyle::KtlintOfficial,
            120,
        )
        .unwrap();

        let after_source = std::fs::read_to_string(f.path()).unwrap();
        let tree2 = parser.parse(&after_source);
        let after = engine.check("test.kt", &tree2, &after_source).len();

        assert!(
            after < before,
            "Format should reduce violations: {} → {}",
            before,
            after
        );
    }

    #[test]
    fn matches_spotless_8_8_0_ktlint_1_8_0_golden() {
        let source = include_str!("../tests/fixtures/spotless-parity/input.kt");
        let expected = include_str!("../tests/fixtures/spotless-parity/expected.kt");
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(source.as_bytes()).unwrap();

        let (_, actual) = lint_and_fix(source, &file);
        assert_eq!(
            actual, expected,
            "formatter output must match Spotless byte-for-byte"
        );
    }

    #[test]
    fn matches_spotless_extended_golden() {
        let source = include_str!("../tests/fixtures/spotless-parity/extended-input.kt");
        let expected = include_str!("../tests/fixtures/spotless-parity/extended-expected.kt");
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(source.as_bytes()).unwrap();

        let (_, actual) = lint_and_fix(source, &file);
        assert_eq!(
            actual, expected,
            "formatter output must match Spotless byte-for-byte"
        );
    }

    #[test]
    fn issue262_raw_string_guard_not_tripped_and_byte_identical() {
        // Issue #262: the indent fixer used to re-indent the closing
        // delimiter row of a raw string that follows a `when`-expression-body
        // function (`    """.trimIndent()` 4 → 8 spaces), corrupting the
        // string content and tripping the protected-region guard, which then
        // aborted the entire `--format` run. The file must pass through
        // auto_fix byte-identical with no error.
        let source = concat!(
            "package com.example\n",
            "\n",
            "internal object Defaults {\n",
            "    fun of(name: Name): String = when (name) {\n",
            "        Name.CORE -> CORE\n",
            "    }\n",
            "\n",
            "    private val CORE = \"\"\"\n",
            "        {\n",
            "          \"a\": \"0.0.0\"\n",
            "        }\n",
            "    \"\"\".trimIndent()\n",
            "}\n",
        );
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(source.as_bytes()).unwrap();
        // auto_fix with the issue's android_studio config (multiline-
        // expression-wrapping is official-only and off here, so the indent
        // fixer is the only one that could touch the raw string).
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        let engine = RuleEngine::new(&KtlintConfig::default());
        let mut violations = engine.check("test.kt", &tree, source);
        for v in &mut violations {
            v.file = f.path().to_string_lossy().to_string();
        }
        formatter::auto_fix(
            &[f.path().to_path_buf()],
            &violations,
            4,
            true,
            &Default::default(),
            crate::config::CodeStyle::AndroidStudio,
            120,
        )
        .unwrap();
        let after = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(after, source, "file must be byte-identical after --format");
    }
}
