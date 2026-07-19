//! standard:no-empty-first-line-in-class-body
//! JVM-compatible: checks all class-like declarations (class, interface, object, enum, etc.).

use crate::rules::{Rule, Violation};

pub struct NoEmptyFirstLineInClassBody;

impl Rule for NoEmptyFirstLineInClassBody {
    fn id(&self) -> &'static str {
        "standard:no-empty-first-line-in-class-body"
    }
    fn check(&self, _t: &tree_sitter::Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let l: Vec<&str> = s.lines().collect();
        for (i, ln) in l.iter().enumerate() {
            let t = ln.trim();
            if t.ends_with('{') && is_class_like_declaration(t) {
                if i + 1 < l.len() && l[i + 1].trim().is_empty() {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 2,
                        col: 1,
                        rule_id: self.id().into(),
                        message: "Unexpected blank line in class body".into(),
                        auto_fixable: true,
                    });
                }
            }
        }
        v
    }
}

/// Check if a trimmed line starts a class-like declaration.
fn is_class_like_declaration(line: &str) -> bool {
    line.contains("class ")
        || line.contains("interface ")
        || line.contains("object ")
        || line.contains("enum class ")
        || line.contains("companion object ")
        || line.contains("data class ")
        || line.contains("sealed class ")
        || line.contains("sealed interface ")
        || line.contains("abstract class ")
        || line.contains("open class ")
        || line.contains("inline class ")
        || line.contains("value class ")
        || line.contains("expect class ")
        || line.contains("actual class ")
        || line.contains("annotation class ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        NoEmptyFirstLineInClassBody.check(&p.parse(s), s)
    }

    #[test]
    fn ok() {
        assert!(c("class Foo {\n    val x=1\n}\n").is_empty());
    }
    #[test]
    fn bad() {
        assert!(!c("class Foo {\n\n    val x=1\n}\n").is_empty());
    }
    #[test]
    fn fun_ignored() {
        assert!(c("fun bar() {\n    return 1\n}\n").is_empty());
    }
    #[test]
    fn data_class_ok() {
        assert!(c("data class Foo(val x: Int)\n").is_empty());
    }
    #[test]
    fn data_class_bad() {
        assert!(!c("data class Foo {\n\n    val x=1\n}\n").is_empty());
    }
    #[test]
    fn enum_class_bad() {
        assert!(!c("enum class Foo {\n\n    A\n}\n").is_empty());
    }
    #[test]
    fn sealed_class_bad() {
        assert!(!c("sealed class Foo {\n\n    class A: Foo()\n}\n").is_empty());
    }
    #[test]
    fn companion_object_bad() {
        assert!(
            !c("class Foo {\n    companion object {\n\n        val x=1\n    }\n}\n").is_empty()
        );
    }
}
