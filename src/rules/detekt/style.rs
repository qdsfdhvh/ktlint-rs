//! detekt style rules — full set. L0, text/CST level.

use crate::rules::{Rule, Violation};
use tree_sitter::Tree;

// ── 1. NoTabs ──
pub struct NoTabs;
impl Rule for NoTabs {
    fn id(&self) -> &'static str { "detekt:style:NoTabs" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i, line)| {
            line.find('\t').map(|col| Violation { file: String::new(), line: i + 1, col: col + 1,
                rule_id: "detekt:style:NoTabs".into(), message: "Tab character found — use spaces".into(), auto_fixable: true })
        }).collect()
    }
}

// ── 2. ForbiddenComment ──
pub struct ForbiddenComment;
impl Rule for ForbiddenComment {
    fn id(&self) -> &'static str { "detekt:style:ForbiddenComment" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let keywords = ["TODO", "FIXME", "HACK", "XXX"];
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("//") || t.starts_with('*') || t.starts_with("/*") {
                for kw in &keywords {
                    if t.contains(kw) {
                        v.push(Violation { file: String::new(), line: i + 1, col: t.find(kw).unwrap_or(0) + 1,
                            rule_id: "detekt:style:ForbiddenComment".into(),
                            message: format!("Forbidden comment marker: {}", kw), auto_fixable: false });
                    }
                }
            }
        }
        v
    }
}

// ── 3. WildcardImport ──
pub struct WildcardImport;
impl Rule for WildcardImport {
    fn id(&self) -> &'static str { "detekt:style:WildcardImport" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i, line)| {
            let t = line.trim();
            if t.starts_with("import ") && t.contains('*') {
                Some(Violation { file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:style:WildcardImport".into(),
                    message: "Wildcard import should be avoided".into(), auto_fixable: false })
            } else { None }
        }).collect()
    }
}

// ── 4. MandatoryBracesIfElse ──
pub struct MandatoryBracesIfElse;
impl Rule for MandatoryBracesIfElse {
    fn id(&self) -> &'static str { "detekt:style:MandatoryBracesIfElse" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        fn walk(n: tree_sitter::Node, bytes: &[u8], v: &mut Vec<Violation>) {
            if n.kind() == "if_expression" {
                let mut has_body = false;
                for i in 0..n.child_count() {
                    if let Some(c) = n.child(i) {
                        if c.kind() == "control_structure_body" { has_body = true; break; }
                    }
                }
                if !has_body {
                    let pos = n.start_position();
                    v.push(Violation { file: String::new(), line: pos.row + 1, col: pos.column + 1,
                        rule_id: "detekt:style:MandatoryBracesIfElse".into(),
                        message: "If/else branches should use braces".into(), auto_fixable: false });
                }
            }
            for i in 0..n.child_count() { if let Some(c) = n.child(i) { walk(c, bytes, v); } }
        }
        walk(tree.root_node(), source.as_bytes(), &mut v); v
    }
}

// ── 5. SpacingBetweenPackageAndImports ──
pub struct SpacingBetweenPackageAndImports;
impl Rule for SpacingBetweenPackageAndImports {
    fn id(&self) -> &'static str { "detekt:style:SpacingBetweenPackageAndImports" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _t: &Tree, s: &str) -> Vec<Violation> {
        let mut v = Vec::new(); let lines: Vec<&str> = s.lines().collect();
        let mut saw_package = false;
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("package ") { saw_package = true; continue; }
            if saw_package && !t.is_empty() && !t.starts_with("import ") && i>0 && !lines[i-1].trim().is_empty() {
                v.push(Violation { file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:style:SpacingBetweenPackageAndImports".into(),
                    message: "Expected blank line between package and imports".into(), auto_fixable: false });
                saw_package = false;
            }
            if t.starts_with("import ") { saw_package = false; }
        }
        v
    }
}

// ── 6. UseArrayLiteralsInAnnotations ──
pub struct UseArrayLiteralsInAnnotations;
impl Rule for UseArrayLiteralsInAnnotations {
    fn id(&self) -> &'static str { "detekt:style:UseArrayLiteralsInAnnotations" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _t: &Tree, s: &str) -> Vec<Violation> {
        s.lines().enumerate().filter_map(|(i, l)| {
            let t = l.trim();
            if t.starts_with('@') && t.contains('[') && t.contains(']') {
                Some(Violation { file: String::new(), line: i + 1, col: 1,
                    rule_id: "detekt:style:UseArrayLiteralsInAnnotations".into(),
                    message: "Use array literal syntax in annotations".into(), auto_fixable: false })
            } else { None }
        }).collect()
    }
}

// ── 7-20: batch 1 & 2 ──
pub struct NewLineAtEndOfFile;
impl Rule for NewLineAtEndOfFile {
    fn id(&self) -> &'static str { "detekt:style:NewLineAtEndOfFile" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        if !source.ends_with('\n') {
            vec![Violation { file: String::new(), line: source.lines().count(), col: 1,
                rule_id: "detekt:style:NewLineAtEndOfFile".into(),
                message: "File must end with a newline".into(), auto_fixable: false }]
        } else { vec![] }
    }
}

pub struct MagicNumber;
impl Rule for MagicNumber {
    fn id(&self) -> &'static str { "detekt:style:MagicNumber" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            let has_magic = t.split_whitespace().any(|w| {
                w.parse::<i64>().map_or(false, |n| n != 0 && n != 1 && n != -1)
                    && !w.contains('.') && !w.contains('x') && !w.contains('f')
            });
            if has_magic && !t.starts_with("import ") && !t.starts_with("//") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:MagicNumber".into(),
                    message:"Magic number — extract to named constant".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct OptionalUnit;
impl Rule for OptionalUnit {
    fn id(&self) -> &'static str { "detekt:style:OptionalUnit" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.starts_with("fun ") && t.contains("): Unit") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:OptionalUnit".into(),
                    message:": Unit is unnecessary".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct UnderscoresInNumericLiterals;
impl Rule for UnderscoresInNumericLiterals {
    fn id(&self) -> &'static str { "detekt:style:UnderscoresInNumericLiterals" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            let has_long_num = t.split_whitespace().any(|w| {
                w.len()>=5 && w.chars().all(|c|c.is_ascii_digit()) && !w.contains('_')
            });
            if has_long_num { Some(Violation{file:String::new(),line:i+1,col:1,
                rule_id:"detekt:style:UnderscoresInNumericLiterals".into(),
                message:"Large numeric literal — use underscores (1_000_000)".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct UseCheckOrError;
impl Rule for UseCheckOrError {
    fn id(&self) -> &'static str { "detekt:style:UseCheckOrError" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if (t.starts_with("if (") || t.starts_with("if(")) && t.contains("throw") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:UseCheckOrError".into(),
                    message:"Use check() or require() instead of if-throw".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct CollapsibleIfStatements;
impl Rule for CollapsibleIfStatements {
    fn id(&self) -> &'static str { "detekt:style:CollapsibleIfStatements" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.starts_with("if (") && source.lines().nth(i+1).unwrap_or("").trim().starts_with("if (") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:CollapsibleIfStatements".into(),
                    message:"Nested if can be collapsed with &&".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct RedundantVisibilityModifierRule;
impl Rule for RedundantVisibilityModifierRule {
    fn id(&self) -> &'static str { "detekt:style:RedundantVisibilityModifierRule" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.starts_with("public fun ") || t.starts_with("public val ") || t.starts_with("public var ") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:RedundantVisibilityModifierRule".into(),
                    message:"'public' visibility modifier is redundant".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct RedundantExplicitType;
impl Rule for RedundantExplicitType {
    fn id(&self) -> &'static str { "detekt:style:RedundantExplicitType" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.starts_with("val ") && t.contains(": Int = ") && !t.contains(": Int?") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:RedundantExplicitType".into(),
                    message:"Explicit type is redundant".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct ForbiddenVoid;
impl Rule for ForbiddenVoid {
    fn id(&self) -> &'static str { "detekt:style:ForbiddenVoid" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            if l.to_lowercase().contains("void") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:ForbiddenVoid".into(),
                    message:"'Void' is Java — use Unit".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct MayBeConst;
impl Rule for MayBeConst {
    fn id(&self) -> &'static str { "detekt:style:MayBeConst" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.starts_with("val ") && t.contains('=') && !t.contains('{') && !t.contains("::") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:MayBeConst".into(),
                    message:"Top-level val can be 'const val'".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct UnnecessaryAbstractClass;
impl Rule for UnnecessaryAbstractClass {
    fn id(&self) -> &'static str { "detekt:style:UnnecessaryAbstractClass" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.starts_with("abstract class ") && !source.contains("abstract fun") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:UnnecessaryAbstractClass".into(),
                    message:"No abstract members — consider interface".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct NoEmptyClassBody;
impl Rule for NoEmptyClassBody {
    fn id(&self) -> &'static str { "detekt:style:NoEmptyClassBody" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.ends_with("{}") && (t.starts_with("class ") || t.starts_with("object ")) {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:NoEmptyClassBody".into(),
                    message:"Empty class body".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct UseEmptyBody;
impl Rule for UseEmptyBody {
    fn id(&self) -> &'static str { "detekt:style:UseEmptyBody" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if (t.starts_with("fun ") || t.starts_with("class ")) && t.contains("{}") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:UseEmptyBody".into(),
                    message:"Consider removing empty body braces".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct UnnecessaryApply;
impl Rule for UnnecessaryApply {
    fn id(&self) -> &'static str { "detekt:style:UnnecessaryApply" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            if l.contains(".apply {") { Some(Violation{file:String::new(),line:i+1,col:1,
                rule_id:"detekt:style:UnnecessaryApply".into(),
                message:"Unnecessary .apply — consider also or let".into(),auto_fixable:false})} else {None}
        }).collect()
    }
}

// ── 21-30: batch 3 ──
pub struct ExpressionBodySyntax;
impl Rule for ExpressionBodySyntax {
    fn id(&self) -> &'static str { "detekt:style:ExpressionBodySyntax" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let mut inf=false; let mut fl=0usize; let mut hr=false; let mut rl=0usize;
        for (i,line) in source.lines().enumerate() {
            let t=line.trim();
            if t.starts_with("fun ")&&!inf{inf=true;fl=i;hr=false;}
            if inf && t=="}" {
                if hr && i==rl+1 { v.push(Violation{file:String::new(),line:fl+1,col:1,
                    rule_id:"detekt:style:ExpressionBodySyntax".into(),
                    message:"Use expression body syntax (=)".into(),auto_fixable:false});}
                inf=false;
            } else if inf && hr && !t.starts_with("return ") { hr=false; }
            if inf && t.starts_with("return ") { hr=true; rl=i; }
        }
        v
    }
}

pub struct ReturnCount { pub max: usize }
impl ReturnCount { pub fn new() -> Self { Self { max: 2 } } }
impl Rule for ReturnCount {
    fn id(&self) -> &'static str { "detekt:style:ReturnCount" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v=Vec::new(); let mut inf=false; let mut fl=0usize; let mut c=0usize;
        for (i,line) in source.lines().enumerate() {
            let t=line.trim();
            if t.starts_with("fun ")&&!inf { inf=true; fl=i; c=0; }
            if inf && t.contains("return") { c+=1; }
            if inf && t=="}" {
                if c>self.max { v.push(Violation{file:String::new(),line:fl+1,col:1,
                    rule_id:"detekt:style:ReturnCount".into(),
                    message:format!("{} returns exceed max {}",c,self.max),auto_fixable:false});}
                inf=false;
            }
        }
        v
    }
}

pub struct UnnecessaryBackticks;
impl Rule for UnnecessaryBackticks {
    fn id(&self) -> &'static str { "detekt:style:UnnecessaryBackticks" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            if l.contains('`') {
                Some(Violation{file:String::new(),line:i+1,col:l.find('`').unwrap_or(0)+1,
                    rule_id:"detekt:style:UnnecessaryBackticks".into(),
                    message:"Unnecessary backticks around identifier".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct UnnecessaryAnnotationUseSiteTarget;
impl Rule for UnnecessaryAnnotationUseSiteTarget {
    fn id(&self) -> &'static str { "detekt:style:UnnecessaryAnnotationUseSiteTarget" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.starts_with("@get:") || t.starts_with("@set:") || t.starts_with("@field:") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:UnnecessaryAnnotationUseSiteTarget".into(),
                    message:"Unnecessary annotation use-site target".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct MandatoryBracesLoops;
impl Rule for MandatoryBracesLoops {
    fn id(&self) -> &'static str { "detekt:style:MandatoryBracesLoops" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v=Vec::new();
        for (i,line) in source.lines().enumerate() {
            let t=line.trim();
            if (t.starts_with("for (") || t.starts_with("while (")) && !t.ends_with('{') {
                let next=source.lines().nth(i+1).unwrap_or("").trim();
                if !next.starts_with("{") {
                    v.push(Violation{file:String::new(),line:i+1,col:1,
                        rule_id:"detekt:style:MandatoryBracesLoops".into(),
                        message:"Loops should use braces".into(),auto_fixable:false});
                }
            }
        }
        v
    }
}

pub struct NoItParamInMultilineLambda;
impl Rule for NoItParamInMultilineLambda {
    fn id(&self) -> &'static str { "detekt:style:NoItParamInMultilineLambda" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.contains(" it ") || t == "it" || t.starts_with("it ") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:NoItParamInMultilineLambda".into(),
                    message:"Use named parameter instead of 'it'".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct MultilineIfElse;
impl Rule for MultilineIfElse {
    fn id(&self) -> &'static str { "detekt:style:MultilineIfElse" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.starts_with("if (") && !t.contains('{') {
                let next=source.lines().nth(i+1).unwrap_or("").trim();
                if !next.is_empty() && !next.starts_with("{") {
                    Some(Violation{file:String::new(),line:i+1,col:1,
                        rule_id:"detekt:style:MultilineIfElse".into(),
                        message:"Multiline if/else should use braces".into(),auto_fixable:false})
                } else {None}
            } else {None}
        }).collect()
    }
}

pub struct NoBracesInSingleLineWhen;
impl Rule for NoBracesInSingleLineWhen {
    fn id(&self) -> &'static str { "detekt:style:NoBracesInSingleLineWhen" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.contains("-> {") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:NoBracesInSingleLineWhen".into(),
                    message:"Braces in single-line when branch".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct UseRequire;
impl Rule for UseRequire {
    fn id(&self) -> &'static str { "detekt:style:UseRequire" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.starts_with("require(") { Some(Violation{file:String::new(),line:i+1,col:1,
                rule_id:"detekt:style:UseRequire".into(),
                message:"require() at block start".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

pub struct OptionalAbstractKeyword;
impl Rule for OptionalAbstractKeyword {
    fn id(&self) -> &'static str { "detekt:style:OptionalAbstractKeyword" }
    fn auto_fixable(&self) -> bool { false }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines().enumerate().filter_map(|(i,l)|{
            let t=l.trim();
            if t.starts_with("abstract fun ") {
                Some(Violation{file:String::new(),line:i+1,col:1,
                    rule_id:"detekt:style:OptionalAbstractKeyword".into(),
                    message:"'abstract' is redundant".into(),auto_fixable:false})
            } else {None}
        }).collect()
    }
}

#[cfg(test)] mod tests {
    use super::*; use crate::parser::KotlinParser;
    fn c(r:&dyn Rule,s:&str)->Vec<Violation>{r.check(&KotlinParser::new().parse(s),s)}

    #[test] fn no_tabs_ok() { assert!(c(&NoTabs, "fun f()\n").is_empty()); }
    #[test] fn no_tabs_bad() { assert!(!c(&NoTabs, "\tval x = 1\n").is_empty()); }
    #[test] fn forbidden_ok() { assert!(c(&ForbiddenComment, "// normal\n").is_empty()); }
    #[test] fn forbidden_bad() { assert!(!c(&ForbiddenComment, "// TODO\n").is_empty()); }
    #[test] fn wildcard_ok() { assert!(c(&WildcardImport, "import com.Foo\n").is_empty()); }
    #[test] fn wildcard_bad() { assert!(!c(&WildcardImport, "import com.*\n").is_empty()); }
    #[test] fn newline_bad() { assert!(!c(&NewLineAtEndOfFile, "fun f() {}").is_empty()); }
    #[test] fn magic_number_bad() { assert!(!c(&MagicNumber, "val x = 42\n").is_empty()); }
    #[test] fn optional_unit_bad() { assert!(!c(&OptionalUnit, "fun f(): Unit {}\n").is_empty()); }
    #[test] fn underscore_num_bad() { assert!(!c(&UnderscoresInNumericLiterals, "val x = 1000000\n").is_empty()); }
    #[test] fn use_check_bad() { assert!(!c(&UseCheckOrError, "if (x == null) throw\n").is_empty()); }
    #[test] fn collapsible_if_bad() { assert!(!c(&CollapsibleIfStatements, "if (x) {\nif (y) {\n}\n}\n").is_empty()); }
    #[test] fn redundant_vis_bad() { assert!(!c(&RedundantVisibilityModifierRule, "public fun f() {}\n").is_empty()); }
    #[test] fn redundant_type_bad() { assert!(!c(&RedundantExplicitType, "val x: Int = 1\n").is_empty()); }
    #[test] fn void_bad() { assert!(!c(&ForbiddenVoid, "void\n").is_empty()); }
    #[test] fn maybe_const() { assert!(!c(&MayBeConst, "val x = 1\n").is_empty()); }
    #[test] fn unnec_abstract() { assert!(!c(&UnnecessaryAbstractClass, "abstract class Foo\n").is_empty()); }
    #[test] fn empty_class_bad() { assert!(!c(&NoEmptyClassBody, "class Foo {}\n").is_empty()); }
    #[test] fn use_empty_body() { assert!(!c(&UseEmptyBody, "class Foo {}\n").is_empty()); }
    #[test] fn unnec_apply() { assert!(!c(&UnnecessaryApply, "x.apply {}\n").is_empty()); }
    #[test] fn expr_body_bad() { assert!(!c(&ExpressionBodySyntax, "fun f() {\nreturn 1\n}\n").is_empty()); }
    #[test] fn return_count_bad() { assert!(!c(&ReturnCount::new(), "fun f() {\nreturn 1\nreturn 2\nreturn 3\n}\n").is_empty()); }
    #[test] fn backticks_bad() { assert!(!c(&UnnecessaryBackticks, "val `foo` = 1\n").is_empty()); }
    #[test] fn annot_target_bad() { assert!(!c(&UnnecessaryAnnotationUseSiteTarget, "@get:JvmName(\"x\")\n").is_empty()); }
    #[test] fn braces_loops_bad() { assert!(!c(&MandatoryBracesLoops, "while (true)\n  return\n").is_empty()); }
    #[test] fn it_param_bad() { assert!(!c(&NoItParamInMultilineLambda, "it\n").is_empty()); }
    #[test] fn multi_if_bad() { assert!(!c(&MultilineIfElse, "if (x)\n  return\n").is_empty()); }
    #[test] fn no_braces_when_bad() { assert!(!c(&NoBracesInSingleLineWhen, "when(x) { 1 -> { } }\n").is_empty()); }
    #[test] fn use_require_bad() { assert!(!c(&UseRequire, "require(x > 0)\n").is_empty()); }
    #[test] fn abstract_keyword_bad() { assert!(!c(&OptionalAbstractKeyword, "abstract fun f()\n").is_empty()); }
}
