//! detekt exceptions rules — exception handling best practices. L0/L1, CST/text level.

use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

// ── InstanceOfCheckForException ──
pub struct InstanceOfCheckForException;
impl Rule for InstanceOfCheckForException {
    fn id(&self) -> &'static str { "detekt:exceptions:InstanceOfCheckForException" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        // Flag `catch (e: Exception) { if (e is ...) ... }` — better to use multi-catch
        let mut v = Vec::new();
        let mut in_catch = false;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("catch (") || t.contains("catch (") { in_catch = true; }
            if in_catch && t.contains(" is ") {
                v.push(Violation { file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:exceptions:InstanceOfCheckForException".into(),
                    message: "Use multi-catch instead of instanceof check in catch".into(),
                    auto_fixable: false,
                });
            }
            if t == "}" && in_catch { in_catch = false; }
        }
        v
    }
}

// ── NotImplementedDeclaration ──
pub struct NotImplementedDeclaration;
impl Rule for NotImplementedDeclaration {
    fn id(&self) -> &'static str { "detekt:exceptions:NotImplementedDeclaration" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i, line)| {
            let t = line.trim();
            if t == "TODO()" || t == "TODO(\"Not yet implemented\")" || t.contains("throw NotImplementedError") ||
               t.contains("error(\"Not implemented\")") {
                Some(Violation { file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:exceptions:NotImplementedDeclaration".into(),
                    message: "TODO() / NotImplementedError should be avoided".into(),
                    auto_fixable: false,
                })
            } else { None }
        }).collect()
    }
}

// ── PrintStackTrace ──
pub struct PrintStackTrace;
impl Rule for PrintStackTrace {
    fn id(&self) -> &'static str { "detekt:exceptions:PrintStackTrace" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i, line)| {
            if line.contains(".printStackTrace()") {
                Some(Violation { file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:exceptions:PrintStackTrace".into(),
                    message: "Avoid printStackTrace() — use logging instead".into(),
                    auto_fixable: false,
                })
            } else { None }
        }).collect()
    }
}

// ── RethrowCaughtException ──
pub struct RethrowCaughtException;
impl Rule for RethrowCaughtException {
    fn id(&self) -> &'static str { "detekt:exceptions:RethrowCaughtException" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_catch = false;
        let mut catch_var = String::new();
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.contains("catch (") && t.contains(':') {
                in_catch = true;
                if let Some(start) = t.find("catch (") {
                    let cap = &t[start+7..];
                    if let Some(end) = cap.find(':') {
                        catch_var = cap[..end].trim().to_string();
                    }
                }
            }
            if in_catch && !catch_var.is_empty() && t.contains(&format!("throw {}", catch_var)) {
                v.push(Violation { file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:exceptions:RethrowCaughtException".into(),
                    message: format!("Caught exception '{}' re-thrown — use throw directly", catch_var),
                    auto_fixable: false,
                });
            }
            if t == "}" && in_catch { in_catch = false; catch_var.clear(); }
        }
        v
    }
}

// ── ReturnFromFinally ──
pub struct ReturnFromFinally;
impl Rule for ReturnFromFinally {
    fn id(&self) -> &'static str { "detekt:exceptions:ReturnFromFinally" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_finally = false;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("} finally {") || t == "finally {" { in_finally = true; }
            if in_finally && t.contains("return") || t.contains("return ") {
                v.push(Violation { file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:exceptions:ReturnFromFinally".into(),
                    message: "Return from finally block suppresses exceptions".into(),
                    auto_fixable: false,
                });
            }
            if in_finally && t == "}" { in_finally = false; }
        }
        v
    }
}

// ── SwallowedException ──
pub struct SwallowedException;
impl Rule for SwallowedException {
    fn id(&self) -> &'static str { "detekt:exceptions:SwallowedException" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.contains("catch (") && t.contains('{') {
                let next = lines.get(i + 1).map(|l| l.trim()).unwrap_or("");
                if next.is_empty() || next == "}" || next == "//" || next.starts_with("//") {
                    v.push(Violation { file: String::new(), line: i + 1, col: 1,
                        rule_id: "detekt:exceptions:SwallowedException".into(),
                        message: "Exception is swallowed — add logging or rethrow".into(),
                        auto_fixable: false,
                    });
                }
            }
        }
        v
    }
}

// ── ThrowingExceptionFromFinally ──
pub struct ThrowingExceptionFromFinally;
impl Rule for ThrowingExceptionFromFinally {
    fn id(&self) -> &'static str { "detekt:exceptions:ThrowingExceptionFromFinally" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_finally = false;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("} finally {") || t == "finally {" { in_finally = true; }
            if in_finally && (t.contains("throw ") || t.contains("error(")) {
                v.push(Violation { file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:exceptions:ThrowingExceptionFromFinally".into(),
                    message: "Throwing from finally block masks original exception".into(),
                    auto_fixable: false,
                });
            }
            if in_finally && t == "}" { in_finally = false; }
        }
        v
    }
}

// ── TooGenericExceptionCaught ──
pub struct TooGenericExceptionCaught;
impl Rule for TooGenericExceptionCaught {
    fn id(&self) -> &'static str { "detekt:exceptions:TooGenericExceptionCaught" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i, line)| {
            let t = line.trim();
            if (t.contains(": Exception)") || t.contains(": Throwable)")) {
                Some(Violation { file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:exceptions:TooGenericExceptionCaught".into(),
                    message: "Catching Exception or Throwable is too generic".into(),
                    auto_fixable: false,
                })
            } else { None }
        }).collect()
    }
}

// ── TooGenericExceptionThrown ──
pub struct TooGenericExceptionThrown;
impl Rule for TooGenericExceptionThrown {
    fn id(&self) -> &'static str { "detekt:exceptions:TooGenericExceptionThrown" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i, line)| {
            let t = line.trim();
            if (t.contains("throw Exception(") || t.contains("throw RuntimeException(") ||
                t.contains("throw Throwable(")) && !t.is_empty() {
                Some(Violation { file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:exceptions:TooGenericExceptionThrown".into(),
                    message: "Throwing generic Exception/Throwable is discouraged".into(),
                    auto_fixable: false,
                })
            } else { None }
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(r: &dyn Rule, s: &str) -> Vec<Violation> { r.check(&KotlinParser::new().parse(s), s) }

    #[test] fn print_stack_bad() {
        assert!(!c(&PrintStackTrace, "e.printStackTrace()\n").is_empty());
    }
    #[test] fn print_stack_ok() {
        assert!(c(&PrintStackTrace, "log.error(e)\n").is_empty());
    }

    #[test] fn not_impl_bad() {
        assert!(!c(&NotImplementedDeclaration, "TODO()\n").is_empty());
    }
    #[test] fn not_impl_ok() {
        assert!(c(&NotImplementedDeclaration, "fun foo() { return 1 }\n").is_empty());
    }

    #[test] fn generic_caught_bad() {
        assert!(!c(&TooGenericExceptionCaught, "catch (e: Exception) { }\n").is_empty());
    }
    #[test] fn generic_caught_ok() {
        assert!(c(&TooGenericExceptionCaught, "catch (e: IOException) { }\n").is_empty());
    }

    #[test] fn generic_thrown_bad() {
        assert!(!c(&TooGenericExceptionThrown, "throw Exception(\"err\")\n").is_empty());
    }

    #[test] fn return_finally_bad() {
        assert!(!c(&ReturnFromFinally, "} finally {\nreturn\n}\n").is_empty());
    }

    #[test] fn throw_finally_bad() {
        assert!(!c(&ThrowingExceptionFromFinally, "} finally {\nthrow e\n}\n").is_empty());
    }

    #[test] fn swallow_bad() {
        assert!(!c(&SwallowedException, "catch (e: Exception) {\n}\n").is_empty());
    }
}
