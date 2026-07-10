//! Ktlint parity tests based on official ktlint documentation examples.
//! Each test follows ktlint's "Ktlint" vs "Disallowed" format.

#[cfg(test)]
mod ktlint_parity {
    use crate::parser::KotlinParser;
    use crate::rules::{RuleEngine, Violation};
    use crate::config::KtlintConfig;

    fn lint(source: &str) -> Vec<Violation> {
        let config = KtlintConfig::default();
        let engine = RuleEngine::new(&config);
        let mut parser = KotlinParser::new();
        let tree = parser.parse(source);
        engine.check("test.kt", &tree, source)
    }

    fn has_rule(v: &[Violation], rule: &str) -> bool { v.iter().any(|x| x.rule_id == rule) }
    fn no_rule(v: &[Violation], rule: &str) -> bool { !has_rule(v, rule) }

    // ── Curly Spacing ──
    #[test] fn curly_allowed() { assert!(no_rule(&lint("val foo = bar { foo() }\n"), "standard:curly-spacing")); }
    #[test] fn curly_disallowed() { assert!(has_rule(&lint("val foo = bar{foo()}\n"), "standard:curly-spacing")); }

    // ── Operator Spacing ──
    #[test] fn op_allowed() { assert!(no_rule(&lint("val foo1 = 1 + 2\nval foo2 = 1 - 2\n"), "standard:op-spacing")); }
    #[test] fn op_disallowed() { assert!(has_rule(&lint("val foo1 = 1+2\n"), "standard:op-spacing")); }

    // ── Comma Spacing ──
    #[test] fn comma_allowed() { assert!(no_rule(&lint("val foo = Foo(1, 3)\n"), "standard:comma-spacing")); }
    #[test] fn comma_disallowed() { assert!(has_rule(&lint("val foo = Foo(1 ,3)\n"), "standard:comma-spacing")); }

    // ── Colon Spacing ──
    #[test] fn colon_allowed() { assert!(no_rule(&lint("class A : B\n"), "standard:colon-spacing")); }
    #[test] fn colon_disallowed() { assert!(has_rule(&lint("class A:B\n"), "standard:colon-spacing")); }

    // ── Double Colon Spacing ──
    #[test] fn double_colon_allowed() { assert!(no_rule(&lint("val foo = Foo::class\n"), "standard:spacing-around-double-colon")); }
    #[test] fn double_colon_disallowed() { assert!(has_rule(&lint("val foo1 = Foo ::class\n"), "standard:spacing-around-double-colon")); }

    // ── Paren Spacing ──
    #[test] fn paren_allowed() { assert!(no_rule(&lint("fun foo(a: Int)\n"), "standard:paren-spacing")); }
    #[test] fn paren_disallowed() { assert!(has_rule(&lint("fun foo( a: Int )\n"), "standard:paren-spacing")); }

    // ── Annotation Spacing ──
    #[test] fn annotation_allowed() {
        assert!(no_rule(&lint("@FunctionalInterface class FooBar {\n    @JvmField var foo: String\n    @Test fun bar() {}\n}\n"), "standard:annotation-spacing"));
    }

    // ── Comment Spacing ──
    #[test] fn comment_allowed() { assert!(no_rule(&lint("// hello\nval x = 1\n"), "standard:comment-spacing")); }
    #[test] fn comment_disallowed() { assert!(has_rule(&lint("//hello\nval x = 1\n"), "standard:comment-spacing")); }

    // ── Indentation ──
    #[test] fn indent_4space_allowed() { assert!(no_rule(&lint("fun f() {\n    val x = 1\n}\n"), "standard:indent")); }
    #[test] fn indent_3space_disallowed() { assert!(has_rule(&lint("fun f() {\n   val x = 1\n}\n"), "standard:indent")); }

    // ── No Wildcard Imports ──
    #[test] fn wildcard_disallowed() { assert!(has_rule(&lint("import java.util.*\n\nclass Foo\n"), "standard:no-wildcard-imports")); }
    #[test] fn import_allowed() { assert!(no_rule(&lint("import java.util.List\n\nclass Foo\n"), "standard:no-wildcard-imports")); }

    // ── No Semicolons ──
    #[test] fn semicolon_disallowed() { assert!(has_rule(&lint("val x = 1;\n"), "standard:no-semicolons")); }
    #[test] fn no_semicolon_allowed() { assert!(no_rule(&lint("val x = 1\n"), "standard:no-semicolons")); }

    // ── Trailing Spaces ──
    #[test] fn trailing_space_disallowed() { assert!(has_rule(&lint("val x = 1   \n"), "standard:no-trailing-spaces")); }

    // ── Final Newline ──
    #[test] fn final_newline_disallowed() { assert!(has_rule(&lint("class Foo"), "standard:final-newline")); }

    // ── Max Line Length ──
    #[test] fn max_line_disallowed() {
        let long = format!("val x = \"{}\"\n", "a".repeat(200));
        assert!(has_rule(&lint(&long), "standard:max-line-length"));
    }

    // ── No Consecutive Blank Lines ──
    #[test] fn consecutive_blank_disallowed() { assert!(has_rule(&lint("a\n\n\nb\n"), "standard:no-consecutive-blank-lines")); }

    // ── No Blank Line Before Rbrace ──
    #[test] fn blank_rbrace_disallowed() { assert!(has_rule(&lint("class Foo {\n\n}\n"), "standard:no-blank-line-before-rbrace")); }

    // ── Chain Wrapping ──
    #[test] fn chain_single_line_allowed() { assert!(no_rule(&lint("list.filter { it > 0 }.map { it * 2 }\n"), "standard:chain-wrapping")); }

    // ── Multiline If/Else ──
    #[test] fn multiline_else_allowed() {
        assert!(no_rule(&lint("if (x) {\n    doA()\n} else {\n    doB()\n}\n"), "standard:multiline-if-else"));
    }

    // ── String Template ──
    #[test] fn string_template_braces_allowed() {
        assert!(no_rule(&lint("val s = \"${name}\"\n"), "standard:string-template"));
    }

    // ── Range Operator Spacing ──
    #[test] fn range_operator_allowed() { assert!(no_rule(&lint("for (i in 1..10)\n"), "standard:spacing-around-range-operator")); }

    // ── Dot Spacing ──
    #[test] fn dot_spacing_disallowed() { assert!(has_rule(&lint("val x = obj . method()\n"), "standard:spacing-around-dot")); }

    // ── No Consecutive Comments ──
    #[test] fn consecutive_comments_disallowed() { assert!(has_rule(&lint("// comment 1\n// comment 2\nval x = 1\n"), "standard:no-consecutive-comments")); }

    // ── Fun Keyword Spacing ──
    #[test] fn fun_keyword_allowed() { assert!(no_rule(&lint("fun foo()\n"), "standard:spacing-after-fun-keyword")); }

    // ── No Single Line Block Comment ──
    #[test] fn single_line_block_comment_disallowed() {
        assert!(has_rule(&lint("/* single line block comment */\nval x = 1\n"), "standard:no-single-line-block-comment"));
    }

    // ── Blank Line Before Declaration ──
    #[test] fn blank_before_decl_disallowed() {
        assert!(has_rule(&lint("val x = 1\nfun bar()\n"), "standard:blank-line-before-declaration"));
    }

    // ── Nullable Type Spacing ──
    #[test] fn nullable_type_allowed() { assert!(no_rule(&lint("val x: String?\n"), "standard:nullable-type-spacing")); }
    #[test] fn nullable_type_disallowed() { assert!(has_rule(&lint("val x: String ?\n"), "standard:nullable-type-spacing")); }

    // ── Property Naming ──
    #[test] fn property_camel_case_allowed() { assert!(no_rule(&lint("val myProperty = 1\n"), "standard:property-naming")); }
    #[test] fn property_const_upper_allowed() { assert!(no_rule(&lint("const val MAX_COUNT = 100\n"), "standard:property-naming")); }

    // ── Class Naming ──
    #[test] fn class_pascal_allowed() { assert!(no_rule(&lint("class MyViewModel\n"), "standard:class-naming")); }

    // ── Function Naming ──
    #[test] fn function_camel_allowed() { assert!(no_rule(&lint("fun myFunction()\n"), "standard:function-naming")); }

    // ── No Empty Class Body ──
    #[test] fn empty_class_body_disallowed() { assert!(has_rule(&lint("class Foo {}\n"), "standard:no-empty-class-body")); }

    // ── Mixed Condition Operators ──
    #[test] fn mixed_condition_disallowed() {
        assert!(has_rule(&lint("if (a && b || c)\n"), "standard:mixed-condition-operators"));
    }
}
