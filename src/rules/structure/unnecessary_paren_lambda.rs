//! standard:unnecessary-parentheses-before-trailing-lambda — remove parens on single lambda arg.
use crate::rules::{Rule, Violation};
pub struct UnnecessaryParenBeforeLambda;
impl Rule for UnnecessaryParenBeforeLambda {
    fn id(&self) -> &'static str {
        "standard:unnecessary-parentheses-before-trailing-lambda"
    }
    fn check(&self, _t: &tree_sitter::Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter(|(_, line)| {
                // Only flag empty parens before a trailing lambda when the
                // callee is a lowercase function-style identifier
                // (`listOf(1).forEach() { }`). Capitalized names are
                // constructor calls (`ViewModel() { }`) where parens are
                // required.
                // Function/class declarations (`fun cleanUp() {`) and
                // constructor calls are not redundant parens.
                let trimmed = line.trim_start();
                // Comments / strings: `() {` inside them is not a call.
                if trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.contains("\"\"\"")
                {
                    return false;
                }
                let paren_in_string = line
                    .find("() {")
                    .is_some_and(|pos| line[..pos].matches('"').count() % 2 == 1);
                if paren_in_string {
                    return false;
                }
                // Getter/setter accessors (`get() {`, incl. inline
                // `val x: Boolean get() {`) — parens are required.
                if trimmed.contains("get() {") || trimmed.contains("set(") {
                    return false;
                }
                // Any function/class declaration (`fun x() {`, `private fun y() {`,
                // `class Z() {`) — the parens belong to the declaration.
                let is_declaration = trimmed
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .take_while(|w| {
                        *w != "fun" && *w != "class" && *w != "interface" && *w != "object"
                    })
                    .last()
                    .is_none()
                    || trimmed.contains(" fun ")
                    || trimmed.contains(" class ")
                    || trimmed.contains(" interface ")
                    || trimmed.contains(" object ")
                    || trimmed.starts_with("class ")
                    || trimmed.starts_with("interface ")
                    || trimmed.starts_with("object ");
                if is_declaration {
                    return false;
                }
                let callee = line
                    .split('(')
                    .next()
                    .and_then(|head| {
                        head.rsplit(|c: char| !c.is_alphanumeric() && c != '_')
                            .next()
                    })
                    .unwrap_or("");
                let lowercase = callee
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_lowercase());
                lowercase && line.contains("() {")
            })
            .map(|(index, _)| Violation {
                file: String::new(),
                line: index + 1,
                col: 1,
                rule_id: self.id().into(),
                message: "Unnecessary parentheses before trailing lambda".into(),
                auto_fixable: true,
            })
            .collect()
    }
}

#[allow(dead_code)]
fn has_empty_parens_before_lambda(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut braces = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if quote.is_none() && byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            break;
        }
        if escaped {
            escaped = false;
        } else if quote.is_some() && byte == b'\\' {
            escaped = true;
        } else if matches!(byte, b'\'' | b'"') {
            if quote == Some(byte) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(byte);
            }
        } else if quote.is_none() && byte == b'{' {
            braces.push(index);
        }
        index += 1;
    }

    for brace in braces {
        let mut close = brace;
        while close > 0 && bytes[close - 1].is_ascii_whitespace() {
            close -= 1;
        }
        if close < 2 || bytes[close - 1] != b')' || bytes[close - 2] != b'(' {
            continue;
        }

        let prefix = &line[..close - 2];
        let is_function_declaration = prefix
            .rsplit_once("fun ")
            .map(|(_, tail)| tail.trim())
            .is_some_and(|tail| {
                let backtick_name = tail.starts_with('`')
                    && tail.ends_with('`')
                    && tail[1..tail.len() - 1].chars().all(|ch| ch != '`');
                backtick_name
                    || (!tail.is_empty()
                        && tail.chars().all(|ch| {
                            ch.is_alphanumeric() || matches!(ch, '_' | '`' | '.' | '<' | '>' | '?')
                        }))
            });
        let is_class_declaration = prefix.split_whitespace().any(|word| {
            matches!(
                word.trim_matches(|ch: char| !ch.is_alphabetic()),
                "class" | "interface" | "object"
            )
        });
        let trimmed_prefix = prefix.trim_end();
        let is_accessor = trimmed_prefix.ends_with("get") || trimmed_prefix.ends_with("set");
        let is_constructor_or_super_call =
            trimmed_prefix.ends_with("constructor") || trimmed_prefix.contains(") :");
        if !is_function_declaration
            && !is_class_declaration
            && !is_accessor
            && !is_constructor_or_super_call
        {
            return true;
        }
    }
    false
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(s: &str) -> Vec<Violation> {
        let mut p = KotlinParser::new();
        UnnecessaryParenBeforeLambda.check(&p.parse(s), s)
    }
    #[test]
    fn good() {
        assert!(c("list.forEach { it }\n").is_empty());
    }
    #[test]
    fn bad() {
        // Redundant empty parens before a trailing lambda on a lowercase
        // function-style callee are reported.
        assert!(!c("list.forEach() { it }\n").is_empty());
        // Constructor calls keep their required parens.
        assert!(c("class Foo : ViewModel() {\n}\n").is_empty());
        assert!(c("SubscriptionViewModel() {\n}\n").is_empty());
    }

    #[test]
    fn function_declarations_are_not_calls() {
        assert!(c("fun render() {\n}\n").is_empty());
        assert!(c("private fun render(items: List<String>) {\n}\n").is_empty());
        assert!(c("fun `descriptive test name`() {\n}\n").is_empty());
        assert!(c("val value get() { return field }\n").is_empty());
        assert!(c(") : Base() {\n}\n").is_empty());
        assert!(c("val sample = \"invoke() {\"\n").is_empty());
        assert!(c("// invoke() {\n").is_empty());
        assert!(c("val raw = \"\"\"invoke() {\"\"\"\n").is_empty());
        assert!(c("class Example : Base() {\n}\n").is_empty());
    }
}
