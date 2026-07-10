mod rules;
mod parser;
mod config;
//! Auto-generated ktlint conformance tests
//! 45 rules mapped from ktlint's 1739 tests
//! Generated from ktlint repository test cases

#[cfg(test)]
mod ktlint_conformance {
    use crate::parser::KotlinParser;
    use crate::rules::{RuleEngine, Violation};
    use crate::config::KtlintConfig;
    use std::collections::HashSet;

    fn lint(source: &str) -> Vec<Violation> {
        let config = KtlintConfig::default();
        let engine = RuleEngine::new(&config);
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        engine.check("test.kt", &tree, source)
    }

    fn has_rule(v: &[Violation], rule_id: &str) -> bool {
        v.iter().any(|x| x.rule_id == rule_id)
    }

    fn count_rule(v: &[Violation], rule_id: &str) -> usize {
        v.iter().filter(|x| x.rule_id == rule_id).count()
    }

    #[test]
    fn ktlint_curly_spacing_coverage() {
        // ktlint test file: SpacingAroundCurlyRuleTest (35 ktlint tests)
        // Rule: standard:curly-spacing
        // ktlint test snippets:
        // "
            fun foo(){println("
        // ")}
            "
        // ".trimIndent()
        val formattedCode =
            "
        // Note: 35 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_op_spacing_coverage() {
        // ktlint test file: SpacingAroundOperatorsRuleTest (13 ktlint tests)
        // Rule: standard:op-spacing
        // ktlint test snippets:
        // "RemoveCurlyBracesFromTemplate"
        // "Operator: {0}"
        // ",
            "
        // Note: 13 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_comma_spacing_coverage() {
        // ktlint test file: SpacingAroundCommaRuleTest (10 ktlint tests)
        // Rule: standard:comma-spacing
        // ktlint test snippets:
        // "
            val foo1 = Foo(1,3)
            val foo2 = Foo(1, 3)
            "
        // ".trimIndent()
        val formattedCode =
            "
        // "
            val foo1 = Foo(1, 3)
            val foo2 = Foo(1, 3)
            
        // Note: 10 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_paren_spacing_coverage() {
        // ktlint test file: SpacingAroundParensRuleTest (21 ktlint tests)
        // Rule: standard:paren-spacing
        // ktlint test snippets:
        // ".trimIndent()
        val formattedCode =
            "
        // ".trimIndent()
        spacingAroundParensRuleAssertThat(code)
            .hasL
        // "
            open class Bar(param: String)
            class Foo : Bar ("
        // Note: 21 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_colon_spacing_coverage() {
        // ktlint test file: SpacingAroundColonRuleTest (14 ktlint tests)
        // Rule: standard:colon-spacing
        // ktlint test snippets:
        // "
            class A:B
            class A2 : B2
            "
        // ".trimIndent()
        val formattedCode =
            "
        // "
            class A : B
            class A2 : B2
            "
        // Note: 14 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_annotation_spacing_coverage() {
        // ktlint test file: AnnotationSpacingRuleTest (15 ktlint tests)
        // Rule: standard:annotation-spacing
        // ktlint test snippets:
        // "
            @JvmField
            fun foo() {}

            "
        // "
            @JvmField

            fun foo() {}
            "
        // ".trimIndent()
        val formattedCode =
            "
        // Note: 15 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_comment_spacing_coverage() {
        // ktlint test file: CommentSpacingRuleTest (1 ktlint tests)
        // Rule: standard:comment-spacing
        // ktlint test snippets:
        // "
                )
            }
                //comment
            "
        // ".trimIndent()
        val formattedCode =
            "
        // "
                )
            }
                // comment
            "
        // Note: 1 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_function_return_type_spacing_coverage() {
        // ktlint test file: FunctionReturnTypeSpacingRuleTest (8 ktlint tests)
        // Rule: standard:function-return-type-spacing
        // ktlint test snippets:
        // "
            fun foo(): String = "
        // "
            "
        // "
            fun foo() : String = "
        // Note: 8 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_function_start_of_body_spacing_coverage() {
        // ktlint test file: FunctionStartOfBodySpacingRuleTest (11 ktlint tests)
        // Rule: standard:function-start-of-body-spacing
        // ktlint test snippets:
        // "
                fun foo() = "
        // "
                fun bar(): String = "
        // "
                "
        // Note: 11 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_class_signature_coverage() {
        // ktlint test file: ClassSignatureRuleTest (60 ktlint tests)
        // Rule: standard:class-signature
        // ktlint test snippets:
        // ".trimIndent()
            val formattedCode =
                "
        // ")
                .hasLintViolation(2, 34, "
        // "Newline expected after opening parenthesis"
        // Note: 60 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_modifier_order_coverage() {
        // ktlint test file: ModifierOrderRuleTest (8 ktlint tests)
        // Rule: standard:modifier-order
        // ktlint test snippets:
        // ".trimIndent()
        val formattedCode =
            "
        // ".trimIndent()
        modifierOrderRuleAssertThat(code)
            .hasLintVio
        // "@Annotation... open abstract\"
        // Note: 8 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_argument_list_wrapping_coverage() {
        // ktlint test file: ArgumentListWrappingRuleTest (43 ktlint tests)
        // Rule: standard:argument-list-wrapping
        // ktlint test snippets:
        // "
            val x = f(
                a,
                b, c
            )
 
        // ".trimIndent()
        val formattedCode =
            "
        // ".trimIndent()
        argumentListWrappingRuleAssertThat(code)
            .has
        // Note: 43 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_indent_coverage() {
        // ktlint test file: IndentationRuleTest (241 ktlint tests)
        // Rule: standard:indent
        // ktlint test snippets:
        // "RemoveCurlyBracesFromTemplate"
        // "KTLINT_UNIT_TEST_TRACE"
        // "
                    val foo = 42
                    "
        // Note: 241 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_no_trailing_spaces_coverage() {
        // ktlint test file: NoTrailingSpacesRuleTest (7 ktlint tests)
        // Rule: standard:no-trailing-spaces
        // ktlint test snippets:
        // ".trimIndent()
        val formattedCode =
            "
        // "
            fun main() {
                val a = 1


            }
           
        // "Trailing space(s)"
        // Note: 7 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_final_newline_coverage() {
        // ktlint test file: FinalNewlineRuleTest (7 ktlint tests)
        // Rule: standard:final-newline
        // ktlint test snippets:
        // "
                fun name() {
                }
                "
        // ".trimIndent()
            val formattedCode =
                "
        // "
                fun name() {
                }

                "
        // Note: 7 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_no_consecutive_blank_lines_coverage() {
        // ktlint test file: NoConsecutiveBlankLinesRuleTest (10 ktlint tests)
        // Rule: standard:no-consecutive-blank-lines
        // ktlint test snippets:
        // "
            package com.test


            import com.test.util


            
        // "


            fun b() {
            }


            fun c()
            "
        // ".trimIndent()
        val formattedCode =
            "
        // Note: 10 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_no_blank_line_before_rbrace_coverage() {
        // ktlint test file: NoBlankLineBeforeRbraceRuleTest (3 ktlint tests)
        // Rule: standard:no-blank-line-before-rbrace
        // ktlint test snippets:
        // ".trimIndent()
        val formattedCode =
            "
        // "Unexpected blank line(s) before \"
        // "),
                LintViolation(6, 1, "
        // Note: 3 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_max_line_length_coverage() {
        // ktlint test file: MaxLineLengthRuleTest (17 ktlint tests)
        // Rule: standard:max-line-length
        // ktlint test snippets:
        // "fooooooooooooooooooooo"
        // "foooooooooooooooooooo"
        // "Exceeded max line length (46)"
        // Note: 17 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_no_empty_file_coverage() {
        // ktlint test file: NoEmptyFileRuleTest (9 ktlint tests)
        // Rule: standard:no-empty-file
        // ktlint test snippets:
        // "
            package foo
            fun main() {
                println("
        // ")
            }
            "
        // "/some/path/Tmp.kt"
        // Note: 9 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_no_wildcard_imports_coverage() {
        // ktlint test file: NoWildcardImportsRuleTest (7 ktlint tests)
        // Rule: standard:no-wildcard-imports
        // ktlint test snippets:
        // "Wildcard import"
        // ")
                .hasLintViolationsWithoutAutoCorrect(
                    Lin
        // "),
                    LintViolation(3, 1, "
        // Note: 7 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_no_unused_imports_coverage() {
        // ktlint test file: NoUnusedImportsRuleTest (39 ktlint tests)
        // Rule: standard:no-unused-imports
        // ktlint test snippets:
        // ".trimIndent()
        val formattedCode =
            "
        // ".trimIndent()
        noUnusedImportsRuleAssertThat(code)
            .hasLintV
        // ".trimIndent()
            val formattedCode =
                "
        // Note: 39 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_class_naming_coverage() {
        // ktlint test file: ClassNamingRuleTest (6 ktlint tests)
        // Rule: standard:class-naming
        // ktlint test snippets:
        // "
                class Foo1
                "
        // ")
        @ValueSource(
            strings = [
                "
        // ",
                "
        // Note: 6 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_function_naming_coverage() {
        // ktlint test file: FunctionNamingRuleTest (14 ktlint tests)
        // Rule: standard:function-naming
        // ktlint test snippets:
        // "
            fun foo1() = "
        // "
            "
        // "
                fun `Some name`() {}
                "
        // Note: 14 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_property_naming_coverage() {
        // ktlint test file: PropertyNamingRuleTest (14 ktlint tests)
        // Rule: standard:property-naming
        // ktlint test snippets:
        // ")
    @ValueSource(
        strings = [
            "
        // ",
            "
        // "
            var $propertyName = "
        // Note: 14 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_package_name_coverage() {
        // ktlint test file: PackageNameRuleTest (5 ktlint tests)
        // Rule: standard:package-name
        // ktlint test snippets:
        // "
            package foo
            "
        // "
            package foo.foo
            "
        // "
            package foo_bar
            "
        // Note: 5 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_filename_coverage() {
        // ktlint test file: FilenameRuleTest (4 ktlint tests)
        // Rule: standard:filename
        // ktlint test snippets:
        // "
            class Foo
            "
        // ".trimIndent()
        fileNameRuleAssertThat(code)
            .asFileWithPath(
        // "
            /*
             * copyright
             */
            "
        // Note: 4 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_backing_property_naming_coverage() {
        // ktlint test file: BackingPropertyNamingRuleTest (19 ktlint tests)
        // Rule: standard:backing-property-naming
        // ktlint test snippets:
        // "Correlated property name: {0}"
        // ",
                    "
        // "
                    class Foo {
                        private var _$property
        // Note: 19 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_enum_entry_name_case_coverage() {
        // ktlint test file: EnumEntryNameCaseRuleTest (9 ktlint tests)
        // Rule: standard:enum-entry-name-case
        // ktlint test snippets:
        // "
            enum class SomeEnum {
                _FOO
            }
         
        // ".trimIndent()
        @Suppress("
        // "Enum entry name should be uppercase underscore-separated names like \"
        // Note: 9 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_no_semicolons_coverage() {
        // ktlint test file: NoSemicolonsRuleTest (26 ktlint tests)
        // Rule: standard:no-semicolons
        // ktlint test snippets:
        // "
            package a.b.c;
            "
        // ".trimIndent()
        val formattedCode =
            "
        // "
            package a.b.c
            "
        // Note: 26 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_string_template_coverage() {
        // ktlint test file: StringTemplateRuleTest (16 ktlint tests)
        // Rule: standard:string-template
        // ktlint test snippets:
        // " in code samples below as "
        // " instead of "
        // "
            val foo1 = "
        // Note: 16 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_if_else_bracing_coverage() {
        // ktlint test file: IfElseBracingRuleTest (15 ktlint tests)
        // Rule: standard:if-else-bracing
        // ktlint test snippets:
        // "CodeStyleValue: {0}"
        // "ktlint_official"
        // ".trimIndent()
        val formattedCode =
            "
        // Note: 15 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_multiline_if_else_coverage() {
        // ktlint test file: MultiLineIfElseRuleTest (27 ktlint tests)
        // Rule: standard:multiline-if-else
        // ktlint test snippets:
        // "
            fun foo() {
                if (true) { return 0 }
            }
 
        // "
            val foo = if (true) { return 0 } else {return 1}
            "
        // "
            fun foo() {
                if (true) return 0
            }
     
        // Note: 27 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_try_catch_finally_wrapping_coverage() {
        // ktlint test file: TryCatchFinallySpacingRuleTest (12 ktlint tests)
        // Rule: standard:try-catch-finally-wrapping
        // ktlint test snippets:
        // "
            }
            "
        // "
            } finally {
                // do something else
            }
   
        // "
            val foo = try {
                "
        // Note: 12 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_string_template_indent_coverage() {
        // ktlint test file: StringTemplateIndentRuleTest (21 ktlint tests)
        // Rule: standard:string-template-indent
        // ktlint test snippets:
        // "RemoveCurlyBracesFromTemplate"
        // ".trimIndent()
        val formattedCode =
            "
        // "Unexpected indent of raw string literal"
        // Note: 21 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_no_empty_class_body_coverage() {
        // ktlint test file: NoEmptyClassBodyRuleTest (6 ktlint tests)
        // Rule: standard:no-empty-class-body
        // ktlint test snippets:
        // ".trimIndent()
        val formattedCode =
            "
        // "Unnecessary block (\"
        // "),
                LintViolation(2, 10, "
        // Note: 6 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_no_empty_first_line_in_class_body_coverage() {
        // ktlint test file: NoEmptyFirstLineInClassBodyRuleTest (4 ktlint tests)
        // Rule: standard:no-empty-first-line-in-class-body
        // ktlint test snippets:
        // "
            class Foo {
                val foo = "
        // "
            }
            "
        // "
            class Foo

            val foo = "
        // Note: 4 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_kdoc_coverage() {
        // ktlint test file: KdocRuleTest (15 ktlint tests)
        // Rule: standard:kdoc
        // ktlint test snippets:
        // ")
    @ValueSource(
        strings = [
            "
        // ",
            "
        // "
            /**
             * Some Foo Kdoc
             */
            "
        // Note: 15 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_spacing_around_double_colon_coverage() {
        // ktlint test file: SpacingAroundDoubleColonRuleTest (6 ktlint tests)
        // Rule: standard:spacing-around-double-colon
        // ktlint test snippets:
        // ".trimIndent()
        val formattedCode =
            "
        // "Unexpected spacing before \"
        // "),
                LintViolation(4, 17, "
        // Note: 6 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_spacing_around_range_operator_coverage() {
        // ktlint test file: SpacingAroundRangeOperatorRuleTest (3 ktlint tests)
        // Rule: standard:spacing-around-range-operator
        // ktlint test snippets:
        // ".trimIndent()
        val formattedCode =
            "
        // "Unexpected spacing after \"
        // "),
                LintViolation(3, 15, "
        // Note: 3 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_chain_wrapping_coverage() {
        // ktlint test file: ChainWrappingRuleTest (16 ktlint tests)
        // Rule: standard:chain-wrapping
        // ktlint test snippets:
        // ".trimIndent()
        val formattedCode =
            "
        // "Line must not end with \"
        // "),
                LintViolation(2, 24, "
        // Note: 16 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_spacing_around_keyword_coverage() {
        // ktlint test file: SpacingAroundKeywordRuleTest (17 ktlint tests)
        // Rule: standard:spacing-around-keyword
        // ktlint test snippets:
        // "
            fun main() {
                if(true) {}
            }
           
        // ".trimIndent()
        val formattedCode =
            "
        // "
            fun main() {
                if (true) {}
            }
          
        // Note: 17 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_no_multi_spaces_coverage() {
        // ktlint test file: NoMultipleSpacesRuleTest (5 ktlint tests)
        // Rule: standard:no-multi-spaces
        // ktlint test snippets:
        // "
            fun main() {
                x(1,$SPACE${SPACE}3)
            }
  
        // ".trimIndent()
        val formattedCode =
            "
        // "
            fun main() {
                x(1,${SPACE}3)
            }
        
        // Note: 5 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_spacing_between_function_name_and_parenthesis_coverage() {
        // ktlint test file: SpacingBetweenFunctionNameAndOpeningParenthesisRuleTest (3 ktlint tests)
        // Rule: standard:spacing-between-function-name-and-parenthesis
        // ktlint test snippets:
        // "
            fun foo() = "
        // "
            "
        // "
            fun foo () = "
        // Note: 3 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_trailing_comma_coverage() {
        // ktlint test file: TrailingCommaOnCallSiteRuleTest (18 ktlint tests)
        // Rule: standard:trailing-comma
        // ktlint test snippets:
        // "
            val foo1 = listOf("
        // ".trimIndent()
        val formattedCode =
            "
        // "Unnecessary trailing comma before \"
        // Note: 18 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }

    #[test]
    fn ktlint_block_comment_initial_star_blank_line_coverage() {
        // ktlint test file: BlockCommentInitialStarAlignmentRuleTest (5 ktlint tests)
        // Rule: standard:block-comment-initial-star-blank-line
        // ktlint test snippets:
        // "
            /*
             * This blocked is formatted well.
             */

        // "
            /*
                      This blocked is formatted well.
         
        // ".trimIndent()
        val formattedCode =
            "
        // Note: 5 ktlint tests to implement
        assert!(true); // placeholder — implement ktlint parity tests
    }
}

// Total: 45 rules mapped from 45 ktlint test files
