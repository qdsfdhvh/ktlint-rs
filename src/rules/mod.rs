//! ktlint rule engine — linting rules for Kotlin code.
//!
//! Each rule is a function that traverses the tree-sitter CST and
//! produces `Violation` structs for style deviations.
//!
//! Rules are categorized:
//! - **spacing**: whitespace around operators, braces, commas, etc.
//! - **indentation**: continuation indents, trailing whitespace
//! - **imports**: wildcard imports, import ordering
//! - **structure**: blank lines, trailing commas, empty files
//! - **wrapping**: chain wrapping, argument wrapping
//! - **naming**: class, function, property, filename conventions

use crate::config::KtlintConfig;
use tree_sitter::Tree;

pub mod imports;
pub mod naming;
pub mod spacing;
pub mod structure;
pub mod suppress;
pub mod wrapping;

// ── Rule trait ──────────────────────────────────────────────────

/// A lint violation found in source code.
#[derive(Debug, Clone)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub rule_id: String,
    pub message: String,
    pub auto_fixable: bool,
}

/// Rule trait — every linter rule implements this.
pub trait Rule: Send + Sync {
    /// Unique rule identifier (e.g. "standard:curly-spacing")
    fn id(&self) -> &'static str;
    /// Whether ktlint can auto-fix this violation
    fn auto_fixable(&self) -> bool {
        true
    }
    /// Check the CST tree and source for violations
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation>;
}

// ── Rule Engine ─────────────────────────────────────────────────

pub struct RuleEngine {
    config: KtlintConfig,
    rules: Vec<Box<dyn Rule>>,
}

impl RuleEngine {
    pub fn new(config: &KtlintConfig) -> Self {
        let rules: Vec<Box<dyn Rule>> = vec![
            // ── Spacing rules (80% of real-world violations) ──
            Box::new(spacing::CurlySpacing),
            Box::new(spacing::OperatorSpacing),
            Box::new(spacing::CommaSpacing),
            Box::new(spacing::ParenSpacing),
            Box::new(spacing::ColonSpacing),
            Box::new(spacing::AnnotationSpacing),
            Box::new(spacing::CommentSpacing),
            Box::new(spacing::FunctionReturnTypeSpacing),
            Box::new(spacing::FunctionStartOfBodySpacing),
            Box::new(spacing::ClassSignatureSpacing),
            // ── Structure rules ──
            Box::new(NoTrailingSpaces),
            Box::new(FinalNewline),
            Box::new(NoConsecutiveBlankLines),
            Box::new(structure::NoBlankLineBeforeRbrace),
            Box::new(structure::Indentation),
            Box::new(structure::MaxLineLength),
            Box::new(structure::NoEmptyFile),
            Box::new(structure::TrailingComma),
            // ── Import rules ──
            Box::new(NoWildcardImports),
            Box::new(imports::ImportOrdering),
            Box::new(imports::NoUnusedImports),
            // ── Wrapping rules ──
            Box::new(wrapping::ChainWrapping),
            Box::new(wrapping::MultilineIfElse),
            Box::new(wrapping::StringTemplateIndent),
            // ── Naming rules ──
            Box::new(naming::ClassNaming),
            Box::new(naming::FunctionNaming),
            Box::new(naming::PropertyNaming),
            Box::new(naming::Filename),
        ];

        Self {
            config: config.clone(),
            rules,
        }
    }

    /// Run all enabled rules on a single file.
    pub fn check(&self, path: &str, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        for rule in &self.rules {
            if !self.config.is_rule_enabled(rule.id()) {
                continue;
            }
            for mut v in rule.check(tree, source) {
                v.file = path.to_string();
                v.auto_fixable = rule.auto_fixable();
                violations.push(v);
            }
        }

        violations
    }
}

// ── Built-in structure rules (simple, kept inline) ─────────────

pub struct NoTrailingSpaces;

impl Rule for NoTrailingSpaces {
    fn id(&self) -> &'static str {
        "standard:no-trailing-spaces"
    }

    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter(|(_, line)| line.ends_with(' ') || line.ends_with('\t'))
            .map(|(i, _)| Violation {
                file: String::new(),
                line: i + 1,
                col: 0,
                rule_id: self.id().to_string(),
                message: "Trailing space(s)".to_string(),
                auto_fixable: true,
            })
            .collect()
    }
}

pub struct FinalNewline;

impl Rule for FinalNewline {
    fn id(&self) -> &'static str {
        "standard:final-newline"
    }

    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        if source.is_empty() || source.ends_with('\n') {
            return vec![];
        }
        vec![Violation {
            file: String::new(),
            line: source.lines().count(),
            col: 0,
            rule_id: self.id().to_string(),
            message: "File must end with a newline".to_string(),
            auto_fixable: true,
        }]
    }
}

pub struct NoConsecutiveBlankLines;

impl Rule for NoConsecutiveBlankLines {
    fn id(&self) -> &'static str {
        "standard:no-consecutive-blank-lines"
    }

    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = vec![];
        let mut blank_count = 0;
        for (i, line) in source.lines().enumerate() {
            if line.trim().is_empty() {
                blank_count += 1;
                if blank_count > 1 {
                    violations.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 0,
                        rule_id: self.id().to_string(),
                        message: "Needless blank line(s)".to_string(),
                        auto_fixable: true,
                    });
                }
            } else {
                blank_count = 0;
            }
        }
        violations
    }
}

pub struct NoWildcardImports;

impl Rule for NoWildcardImports {
    fn id(&self) -> &'static str {
        "standard:no-wildcard-imports"
    }

    fn auto_fixable(&self) -> bool {
        false
    }

    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter(|(_, line)| {
                let trimmed = line.trim();
                trimmed.starts_with("import ") && trimmed.contains(".*")
            })
            .map(|(i, _)| Violation {
                file: String::new(),
                line: i + 1,
                col: 0,
                rule_id: self.id().to_string(),
                message: "Wildcard import".to_string(),
                auto_fixable: false,
            })
            .collect()
    }
}
