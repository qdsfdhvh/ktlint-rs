//! ktlint rule engine — linting rules for Kotlin code.

use crate::config::KtlintConfig;
use tree_sitter::Tree;

pub mod compat;
pub mod imports;
pub mod naming;
pub mod new_rules;
pub mod new_rules2;
pub mod new_rules3;
pub mod new_rules4;
pub mod spacing;
pub mod structure;
pub mod suppress;
pub mod wrapping;
pub mod phase1_more;
pub mod phase1_rules;
pub mod phase3b_rules;
pub use phase3b_rules::*;
pub mod final_rules;
pub use final_rules::*;

#[derive(Debug, Clone)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub rule_id: String,
    pub message: String,
    pub auto_fixable: bool,
}

pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn auto_fixable(&self) -> bool {
        true
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation>;
}

pub struct RuleEngine {
    config: KtlintConfig,
    rules: Vec<Box<dyn Rule>>,
}

impl RuleEngine {
    pub fn new(config: &KtlintConfig) -> Self {
        let rules: Vec<Box<dyn Rule>> = vec![
            // ── Built-in ─────────────────────────────────────────────
            Box::new(NoTrailingSpaces),
            Box::new(FinalNewline),
            Box::new(NoConsecutiveBlankLines),
            // ── Spacing (17 rules) ───────────────────────────────────
            Box::new(spacing::AnnotationSpacing),
            Box::new(spacing::ArgumentListWrapping),
            Box::new(spacing::BlockCommentStar),
            Box::new(spacing::ClassSignatureSpacing),
            Box::new(spacing::ColonSpacing),
            Box::new(spacing::CommaSpacing),
            Box::new(spacing::CommentSpacing),
            Box::new(spacing::CurlySpacing),
            Box::new(spacing::DoubleColonSpacing),
            Box::new(spacing::FunctionNameParenSpacing),
            Box::new(spacing::FunctionReturnTypeSpacing),
            Box::new(spacing::FunctionStartOfBodySpacing),
            Box::new(spacing::ModifierOrder),
            Box::new(spacing::OperatorSpacing),
            Box::new(spacing::ParenSpacing),
            Box::new(spacing::RangeOperatorSpacing),
            Box::new(spacing::SpacingAroundKeyword),
            // ── Structure (27 rules) ──────────────────────────────────
            Box::new(structure::BlankLineBeforeDeclaration),
            Box::new(structure::EnumEntry),
            Box::new(structure::IJTrailingComma),
            Box::new(structure::Indentation::new(config.indent_size)),
            Box::new(structure::KdocFormatting),
            Box::new(structure::KdocNoEmptyFirstLine),
            Box::new(structure::KdocNoTrailingSpace),
            Box::new(structure::LambdaParen),
            Box::new(structure::MaxLineLength),
            Box::new(structure::NoBlankAfterKdoc),
            Box::new(structure::NoBlankBeforeListClose),
            Box::new(structure::NoBlankLineBeforeRbrace),
            Box::new(structure::NoBlankLineInList),
            Box::new(structure::NoEmptyClassBody),
            Box::new(structure::NoEmptyFile),
            Box::new(structure::NoEmptyFileBody),
            Box::new(structure::NoEmptyFirstLineInClassBody),
            Box::new(structure::NoLeadingEmptyLinesInMethod),
            Box::new(structure::NoMultiSpaces),
            Box::new(structure::NoSemicolons),
            Box::new(structure::NoSingleExpressionBody),
            Box::new(structure::NoTrailingSpacesInString),
            Box::new(structure::ParameterListSpacing),
            Box::new(structure::SpacingBetweenDeclarations),
            Box::new(structure::TrailingComma),
            Box::new(structure::TrailingSpacesInComment),
            Box::new(structure::UnnecessaryParenBeforeLambda),
            // ── Imports (4 rules) ─────────────────────────────────────
            Box::new(NoWildcardImports),
            Box::new(imports::ImportOrdering),
            Box::new(imports::NoUnusedImports),
            Box::new(imports::NoWildcardImportsEither),
            // ── Naming (6 rules) ──────────────────────────────────────
            Box::new(naming::BackingPropertyNaming),
            Box::new(naming::ClassNaming),
            Box::new(naming::Filename),
            Box::new(naming::FunctionNaming),
            Box::new(naming::PackageName),
            Box::new(naming::PropertyNaming),
            // ── Wrapping (7 rules) ────────────────────────────────────
            Box::new(wrapping::ChainWrapping),
            Box::new(wrapping::GeneralWrapping),
            Box::new(wrapping::MultilineExpressionWrapping),
            Box::new(wrapping::MultilineIfElse),
            Box::new(wrapping::StringTemplateIndent),
            Box::new(wrapping::TryCatchFinallyWrapping),
            Box::new(wrapping::WhenExpressionLineBreak),
            // ── Phase 1 rules (unique IDs) ────────────────────────────
            Box::new(phase1_rules::WhenEntryBracing),
            Box::new(phase1_rules::BlankLineBetweenWhenConditions),
            Box::new(phase1_rules::SpacingBetweenDeclarationsWithComments),
            // ── Phase 1 more (unique IDs) ─────────────────────────────
            Box::new(phase1_more::KtlintAnnotation),
            Box::new(phase1_more::KtlintWrapping),
            Box::new(phase1_more::KtlintNoConsecutiveComments),
            // ── Phase 3b (unique IDs) ─────────────────────────────────
            Box::new(phase3b_rules::FunctionSignatureSpacing),
            Box::new(phase3b_rules::FunctionExpressionBody),
            Box::new(phase3b_rules::KeywordSpacing),
            // ── Final rules ───────────────────────────────────────────
            Box::new(final_rules::TypeArgumentListSpacing),
            Box::new(final_rules::SpacingAroundAngleBrackets),
            Box::new(final_rules::EnumWrapping),
            Box::new(final_rules::TrailingCommaOnDeclarationSite),
            Box::new(final_rules::TrailingCommaOnCallSite),
        ];
        Self {
            config: config.clone(),
            rules,
        }
    }

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

// ── Built-in simple rules ────────────────────────────────────────────

pub struct NoTrailingSpaces;
impl Rule for NoTrailingSpaces {
    fn id(&self) -> &'static str {
        "standard:no-trailing-spaces"
    }
    fn check(&self, _: &Tree, s: &str) -> Vec<Violation> {
        s.lines()
            .enumerate()
            .filter(|(_, l)| l.ends_with(' ') || l.ends_with('\t'))
            .map(|(i, _)| Violation {
                file: String::new(),
                line: i + 1,
                col: 0,
                rule_id: self.id().into(),
                message: "Trailing space(s)".into(),
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
    fn check(&self, _: &Tree, s: &str) -> Vec<Violation> {
        if s.is_empty() || s.ends_with('\n') {
            vec![]
        } else {
            vec![Violation {
                file: String::new(),
                line: s.lines().count(),
                col: 0,
                rule_id: self.id().into(),
                message: "File must end with a newline".into(),
                auto_fixable: true,
            }]
        }
    }
}
pub struct NoConsecutiveBlankLines;
impl Rule for NoConsecutiveBlankLines {
    fn id(&self) -> &'static str {
        "standard:no-consecutive-blank-lines"
    }
    fn check(&self, _: &Tree, s: &str) -> Vec<Violation> {
        let mut v = vec![];
        let mut b = 0;
        for (i, l) in s.lines().enumerate() {
            if l.trim().is_empty() {
                b += 1;
                if b > 1 {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 0,
                        rule_id: self.id().into(),
                        message: "Needless blank line(s)".into(),
                        auto_fixable: true,
                    });
                }
            } else {
                b = 0;
            }
        }
        v
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
    fn check(&self, _: &Tree, s: &str) -> Vec<Violation> {
        s.lines()
            .enumerate()
            .filter(|(_, l)| {
                let t = l.trim();
                t.starts_with("import ") && t.contains(".*")
            })
            .map(|(i, _)| Violation {
                file: String::new(),
                line: i + 1,
                col: 0,
                rule_id: self.id().into(),
                message: "Wildcard import".into(),
                auto_fixable: false,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_trailing_spaces_detects() {
        let rule = NoTrailingSpaces;
        let violations = rule.check(
            &crate::parser::KotlinParser::new().parse("fun test() \n"),
            "fun test() \n",
        );
        assert!(!violations.is_empty());
    }

    #[test]
    fn final_newline_missing() {
        let rule = FinalNewline;
        let v = rule.check(
            &crate::parser::KotlinParser::new().parse("fun a() {}"),
            "fun a() {}",
        );
        assert!(!v.is_empty());
    }
}
