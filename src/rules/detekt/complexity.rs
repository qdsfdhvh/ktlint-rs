//! detekt complexity rules — code complexity checks. L0/L1, CST/text level.

use crate::rules::{Rule, Violation};
use std::collections::HashMap;
use tree_sitter::Tree;

// ── CyclomaticComplexMethod ──
pub struct CyclomaticComplexMethod {
    pub threshold: usize,
}
impl CyclomaticComplexMethod {
    pub fn new() -> Self {
        Self { threshold: 10 }
    }
}
impl Rule for CyclomaticComplexMethod {
    fn id(&self) -> &'static str {
        "detekt:complexity:CyclomaticComplexMethod"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_cyclo(tree.root_node(), &mut v, self.threshold);
        v
    }
}
fn walk_cyclo(n: tree_sitter::Node, v: &mut Vec<Violation>, t: usize) {
    if n.kind() == "function_body" {
        let ccn = cyclomatic(&n);
        if ccn > t {
            let pos = n.start_position();
            v.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: "detekt:complexity:CyclomaticComplexMethod".into(),
                message: format!("Cyclomatic complexity {} exceeds threshold of {}", ccn, t),
                auto_fixable: false,
            });
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            walk_cyclo(c, v, t);
        }
    }
}
fn cyclomatic(n: &tree_sitter::Node) -> usize {
    1 + (0..n.child_count())
        .map(|i| n.child(i))
        .filter(|c| {
            c.map_or(false, |c| {
                matches!(
                    c.kind(),
                    "if_expression"
                        | "when_entry"
                        | "while_statement"
                        | "for_statement"
                        | "do_while_statement"
                        | "catch_block"
                        | "conjunction_expression"
                        | "disjunction_expression"
                )
            })
        })
        .count()
}

// ── LongMethod ──
pub struct LongMethod {
    pub threshold: usize,
}
impl LongMethod {
    pub fn new() -> Self {
        Self { threshold: 60 }
    }
}
impl Rule for LongMethod {
    fn id(&self) -> &'static str {
        "detekt:complexity:LongMethod"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_fn = false;
        let mut fn_start = 0usize;
        let mut fn_name = String::new();
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("fun ") && !in_fn {
                in_fn = true;
                fn_start = i;
                fn_name = t[4..].split('(').next().unwrap_or("").trim().to_string();
            }
            if t == "}" && in_fn {
                let len = i.saturating_sub(fn_start) + 1;
                if len > self.threshold {
                    v.push(Violation {
                        file: String::new(),
                        line: fn_start + 1,
                        col: 1,
                        rule_id: "detekt:complexity:LongMethod".into(),
                        message: format!(
                            "Method \"{}\" has {} lines, exceeding threshold of {}",
                            fn_name, len, self.threshold
                        ),
                        auto_fixable: false,
                    });
                }
                in_fn = false;
            }
        }
        v
    }
}

// ── LongParameterList ──
pub struct LongParameterList {
    pub threshold: usize,
}
impl LongParameterList {
    pub fn new() -> Self {
        Self { threshold: 6 }
    }
}
impl Rule for LongParameterList {
    fn id(&self) -> &'static str {
        "detekt:complexity:LongParameterList"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_params(tree.root_node(), &mut v, self.threshold);
        v
    }
}
fn walk_params(n: tree_sitter::Node, v: &mut Vec<Violation>, t: usize) {
    if n.kind() == "function_declaration" || n.kind() == "constructor_declaration" {
        let mut count = 0usize;
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i) {
                if c.kind() == "parameter" || c.kind() == "class_parameter" {
                    count += 1;
                }
            }
        }
        if count > t {
            let pos = n.start_position();
            v.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: "detekt:complexity:LongParameterList".into(),
                message: format!("{} parameters exceed threshold of {}", count, t),
                auto_fixable: false,
            });
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            walk_params(c, v, t);
        }
    }
}

// ── TooManyFunctions ──
pub struct TooManyFunctions {
    pub threshold: usize,
}
impl TooManyFunctions {
    pub fn new() -> Self {
        Self { threshold: 25 }
    }
}
impl Rule for TooManyFunctions {
    fn id(&self) -> &'static str {
        "detekt:complexity:TooManyFunctions"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let bytes = source.as_bytes();
        let mut per_class: Vec<(String, usize, usize)> = Vec::new();
        count_fns(tree.root_node(), bytes, &mut per_class);
        for (name, count, line) in &per_class {
            if *count > self.threshold {
                v.push(Violation {
                    file: String::new(),
                    line: *line,
                    col: 1,
                    rule_id: "detekt:complexity:TooManyFunctions".into(),
                    message: format!(
                        "{} has {} functions, exceeding threshold of {}",
                        name, count, self.threshold
                    ),
                    auto_fixable: false,
                });
            }
        }
        v
    }
}
fn count_fns(n: tree_sitter::Node, bytes: &[u8], per_class: &mut Vec<(String, usize, usize)>) {
    if n.kind() == "class_body" || n.kind() == "object_body" {
        let mut count = 0usize;
        let line = n.start_position().row + 1;
        let mut name = String::new();
        if let Some(p) = n.parent() {
            for i in 0..p.child_count() {
                if let Some(c) = p.child(i) {
                    if c.kind() == "simple_identifier" {
                        if let Ok(nm) = c.utf8_text(bytes) {
                            name = nm.to_string();
                        }
                        break;
                    }
                }
            }
        }
        for i in 0..n.child_count() {
            if let Some(c) = n.child(i) {
                if c.kind() == "function_declaration" {
                    count += 1;
                }
            }
        }
        if !name.is_empty() {
            per_class.push((name, count, line));
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            count_fns(c, bytes, per_class);
        }
    }
}

// ── NestedBlockDepth ──
pub struct NestedBlockDepth {
    pub threshold: usize,
}
impl NestedBlockDepth {
    pub fn new() -> Self {
        Self { threshold: 4 }
    }
}
impl Rule for NestedBlockDepth {
    fn id(&self) -> &'static str {
        "detekt:complexity:NestedBlockDepth"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut depth = 0u32;
        let mut max = 0u32;
        let mut max_line = 1usize;
        for (i, line) in source.lines().enumerate() {
            for ch in line.chars() {
                if ch == '{' {
                    depth += 1;
                    if depth > max {
                        max = depth;
                        max_line = i + 1;
                    }
                } else if ch == '}' && depth > 0 {
                    depth -= 1;
                }
            }
        }
        if max as usize > self.threshold {
            v.push(Violation {
                file: String::new(),
                line: max_line,
                col: 1,
                rule_id: "detekt:complexity:NestedBlockDepth".into(),
                message: format!(
                    "Nesting depth {} exceeds threshold of {}",
                    max, self.threshold
                ),
                auto_fixable: false,
            });
        }
        v
    }
}

// ── LargeClass ──
pub struct LargeClass {
    pub threshold: usize,
}
impl LargeClass {
    pub fn new() -> Self {
        Self { threshold: 150 }
    }
}
impl Rule for LargeClass {
    fn id(&self) -> &'static str {
        "detekt:complexity:LargeClass"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_class = false;
        let mut start = 0usize;
        let mut name = String::new();
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if (t.starts_with("class ") || t.starts_with("object ")) && t.contains('{') {
                in_class = true;
                start = i;
                name = t[..t.find('{').unwrap_or(t.len())].trim().to_string();
            }
            if in_class && t == "}" {
                let len = i.saturating_sub(start) + 1;
                if len > self.threshold {
                    v.push(Violation {
                        file: String::new(),
                        line: start + 1,
                        col: 1,
                        rule_id: "detekt:complexity:LargeClass".into(),
                        message: format!(
                            "{} has {} lines, exceeding threshold of {}",
                            name, len, self.threshold
                        ),
                        auto_fixable: false,
                    });
                }
                in_class = false;
            }
        }
        v
    }
}

// ── ComplexCondition ──
pub struct ComplexCondition {
    pub threshold: usize,
}
impl ComplexCondition {
    pub fn new() -> Self {
        Self { threshold: 4 }
    }
}
impl Rule for ComplexCondition {
    fn id(&self) -> &'static str {
        "detekt:complexity:ComplexCondition"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("if (") || t.starts_with("} else if (") || t.contains("while (") {
                let booleans = t.matches("&&").count() + t.matches("||").count() + 1;
                if booleans > self.threshold {
                    v.push(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:complexity:ComplexCondition".into(),
                        message: format!("Condition has {} boolean operators", booleans),
                        auto_fixable: false,
                    });
                }
            }
        }
        v
    }
}

// ── StringLiteralDuplication ──
pub struct StringLiteralDuplication {
    threshold: usize,
}
impl StringLiteralDuplication {
    pub fn new() -> Self {
        Self { threshold: 3 }
    }
}
impl Rule for StringLiteralDuplication {
    fn id(&self) -> &'static str {
        "detekt:complexity:StringLiteralDuplication"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut seen: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, line) in source.lines().enumerate() {
            let mut start = 0usize;
            while let Some(s) = line[start..].find('"') {
                let abs = start + s;
                if let Some(e) = line[abs + 1..].find('"') {
                    let lit = &line[abs..=abs + e + 1];
                    if lit.len() > 2 {
                        seen.entry(lit).or_default().push(i + 1);
                    }
                    start = abs + e + 2;
                } else {
                    break;
                }
            }
        }
        for (lit, lines) in &seen {
            if lines.len() >= self.threshold {
                v.push(Violation {
                    file: String::new(),
                    line: lines[0],
                    col: 1,
                    rule_id: "detekt:complexity:StringLiteralDuplication".into(),
                    message: format!("String literal {} duplicated {} times", lit, lines.len()),
                    auto_fixable: false,
                });
            }
        }
        v
    }
}

// ── CognitiveComplexMethod ──
pub struct CognitiveComplexMethod {
    pub threshold: usize,
}
impl CognitiveComplexMethod {
    pub fn new() -> Self {
        Self { threshold: 15 }
    }
}
impl Rule for CognitiveComplexMethod {
    fn id(&self) -> &'static str {
        "detekt:complexity:CognitiveComplexMethod"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_cognitive(tree.root_node(), &mut v, self.threshold);
        v
    }
}
fn walk_cognitive(n: tree_sitter::Node, v: &mut Vec<Violation>, t: usize) {
    if n.kind() == "function_body" {
        let score = cognitive_score(&n);
        if score > t {
            let pos = n.start_position();
            v.push(Violation {
                file: String::new(),
                line: pos.row + 1,
                col: pos.column + 1,
                rule_id: "detekt:complexity:CognitiveComplexMethod".into(),
                message: format!("Cognitive complexity {} exceeds threshold of {}", score, t),
                auto_fixable: false,
            });
        }
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            walk_cognitive(c, v, t);
        }
    }
}
fn cognitive_score(n: &tree_sitter::Node) -> usize {
    let base = match n.kind() {
        "if_expression" | "when_entry" | "while_statement" | "for_statement"
        | "do_while_statement" | "catch_block" => 1,
        "conjunction_expression" | "disjunction_expression" => 1,
        _ if n.kind().ends_with("_expression") && n.kind().contains("elvis") => 1,
        "jump_expression" => 1,
        _ => 0,
    };
    let nesting = if matches!(
        n.kind(),
        "if_expression"
            | "while_statement"
            | "for_statement"
            | "do_while_statement"
            | "when_expression"
            | "try_expression"
            | "lambda_literal"
    ) {
        1
    } else {
        0
    };
    let child: usize = (0..n.child_count())
        .map(|i| n.child(i).map_or(0, |c| cognitive_score(&c)))
        .sum();
    base + nesting + child
}

// ── MethodOverloading ──
pub struct MethodOverloading {
    pub threshold: usize,
}
impl MethodOverloading {
    pub fn new() -> Self {
        Self { threshold: 5 }
    }
}
impl Rule for MethodOverloading {
    fn id(&self) -> &'static str {
        "detekt:complexity:MethodOverloading"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_class = false;
        let mut depth = 0u32;
        let mut class_names: HashMap<String, (usize, usize)> = HashMap::new();
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.contains("class ") || t.contains("object ") || t.contains("enum class ") {
                in_class = true;
                class_names.clear();
                depth = 0;
            }
            if in_class {
                for ch in t.chars() {
                    if ch == '{' {
                        depth += 1;
                    } else if ch == '}' && depth > 0 {
                        depth -= 1;
                    }
                }
                if depth == 0 && t.contains('}') {
                    in_class = false;
                }
                if t.starts_with("fun ") {
                    let rest = &t[4..];
                    if let Some(paren) = rest.find('(') {
                        let name = rest[..paren].trim();
                        if !name.is_empty() && !name.contains(' ') {
                            let entry = class_names.entry(name.to_string()).or_insert((i + 1, 0));
                            entry.1 += 1;
                        }
                    }
                }
            }
        }
        for (name, (line, count)) in &class_names {
            if *count > self.threshold {
                v.push(Violation {
                    file: String::new(),
                    line: *line,
                    col: 1,
                    rule_id: "detekt:complexity:MethodOverloading".into(),
                    message: format!(
                        "Method \"{}\" has {} overloads, exceeding threshold of {}",
                        name, count, self.threshold
                    ),
                    auto_fixable: false,
                });
            }
        }
        v
    }
}

// ── NamedArguments ──
pub struct NamedArguments {
    pub threshold: usize,
}
impl NamedArguments {
    pub fn new() -> Self {
        Self { threshold: 3 }
    }
}
impl Rule for NamedArguments {
    fn id(&self) -> &'static str {
        "detekt:complexity:NamedArguments"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            for (paren_pos, _) in t.match_indices('(') {
                if let Some(paren_close) = t[paren_pos..].find(')') {
                    let args = &t[paren_pos + 1..paren_pos + paren_close];
                    if args.trim().is_empty() {
                        continue;
                    }
                    let unnamed = args
                        .split(',')
                        .filter(|p| !p.trim().is_empty() && !p.contains('='))
                        .count();
                    if unnamed > self.threshold {
                        let before = t[..paren_pos].trim();
                        let last_word = before.split_whitespace().last().unwrap_or("");
                        let skip = ["fun", "class", "if", "when", "for", "while", "catch"];
                        if !before.is_empty()
                            && !skip.contains(&last_word)
                            && last_word.chars().any(|c| c.is_alphanumeric())
                        {
                            v.push(Violation {
                                file: String::new(),
                                line: i + 1,
                                col: paren_pos + 1,
                                rule_id: "detekt:complexity:NamedArguments".into(),
                                message: format!(
                                    "Call has {} positional arguments, exceeding threshold of {}",
                                    unnamed, self.threshold
                                ),
                                auto_fixable: false,
                            });
                        }
                    }
                }
            }
        }
        v
    }
}

// ── SpreadOperator ──
pub struct SpreadOperator;
impl Rule for SpreadOperator {
    fn id(&self) -> &'static str {
        "detekt:complexity:SpreadOperator"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, tree: &Tree, _s: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        walk_spread(tree.root_node(), &mut v);
        v
    }
}
fn walk_spread(n: tree_sitter::Node, v: &mut Vec<Violation>) {
    if n.kind() == "spread_expression" {
        let pos = n.start_position();
        v.push(Violation {
            file: String::new(),
            line: pos.row + 1,
            col: pos.column + 1,
            rule_id: "detekt:complexity:SpreadOperator".into(),
            message: "Spread operator usage should be avoided".into(),
            auto_fixable: false,
        });
    }
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) {
            walk_spread(c, v);
        }
    }
}

// ── ComplexInterface ──
pub struct ComplexInterface {
    pub threshold: usize,
}
impl ComplexInterface {
    pub fn new() -> Self {
        Self { threshold: 10 }
    }
}
impl Rule for ComplexInterface {
    fn id(&self) -> &'static str {
        "detekt:complexity:ComplexInterface"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        let mut v = Vec::new();
        let mut in_interface = false;
        let mut member_count = 0usize;
        let mut start_line = 0usize;
        let mut depth = 0u32;
        for (i, line) in source.lines().enumerate() {
            let t = line.trim();
            if t.contains("interface ") {
                in_interface = true;
                member_count = 0;
                start_line = i + 1;
            }
            if in_interface {
                for ch in t.chars() {
                    if ch == '{' {
                        depth += 1;
                    } else if ch == '}' && depth > 0 {
                        depth -= 1;
                    }
                }
                if depth > 0
                    && (t.starts_with("fun ") || t.starts_with("val ") || t.starts_with("var "))
                    && !t.contains('=')
                    && !t.contains('{')
                {
                    member_count += 1;
                }
                if depth == 0 && t.contains('}') {
                    if member_count > self.threshold {
                        v.push(Violation {
                            file: String::new(),
                            line: start_line,
                            col: 1,
                            rule_id: "detekt:complexity:ComplexInterface".into(),
                            message: format!(
                                "Interface has {} members, exceeding threshold of {}",
                                member_count, self.threshold
                            ),
                            auto_fixable: false,
                        });
                    }
                    in_interface = false;
                }
            }
        }
        v
    }
}

// ── LabeledExpression ──
pub struct LabeledExpression;
impl Rule for LabeledExpression {
    fn id(&self) -> &'static str {
        "detekt:complexity:LabeledExpression"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, line)| {
                let t = line.trim();
                let has_label = t.contains('@')
                    && (t.starts_with("return@")
                        || t.starts_with("break@")
                        || t.starts_with("continue@")
                        || t.split_whitespace().any(|w| w.ends_with('@')));
                if has_label {
                    Some(Violation {
                        file: String::new(),
                        line: i + 1,
                        col: 1,
                        rule_id: "detekt:complexity:LabeledExpression".into(),
                        message: "Labeled expressions reduce readability".into(),
                        auto_fixable: false,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── ReplaceSafeCallChainWithRun ──
pub struct ReplaceSafeCallChainWithRun;
impl Rule for ReplaceSafeCallChainWithRun {
    fn id(&self) -> &'static str {
        "detekt:complexity:ReplaceSafeCallChainWithRun"
    }
    fn auto_fixable(&self) -> bool {
        false
    }
    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, line)| {
                let t = line.trim();
                for p in &["?.let {", "?.run {", "?.also {", "?.apply {"] {
                    if t.contains(p) && !t.starts_with("import ") {
                        return Some(Violation {
                            file: String::new(),
                            line: i + 1,
                            col: 1,
                            rule_id: "detekt:complexity:ReplaceSafeCallChainWithRun".into(),
                            message: "Consider replacing safe-call chain with run { }".into(),
                            auto_fixable: false,
                        });
                    }
                }
                None
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::KotlinParser;
    fn c(r: &dyn Rule, s: &str) -> Vec<Violation> {
        r.check(&KotlinParser::new().parse(s), s)
    }

    #[test]
    fn cyclo_ok() {
        assert!(c(&CyclomaticComplexMethod::new(), "fun f() {}\n").is_empty());
    }
    #[test]
    fn long_ok() {
        assert!(c(&LongMethod::new(), "fun f() { val x = 1\n}\n").is_empty());
    }
    #[test]
    fn long_bad() {
        let mut s = String::from("fun f() {\n");
        for _ in 0..60 {
            s.push_str("    x\n");
        }
        s.push_str("}\n");
        assert!(!c(&LongMethod { threshold: 30 }, &s).is_empty());
    }
    #[test]
    fn params_ok() {
        assert!(c(&LongParameterList::new(), "fun f(a:Int)\n").is_empty());
    }
    #[test]
    fn too_many_fn_ok() {
        assert!(c(&TooManyFunctions::new(), "class C{}\n").is_empty());
    }
    #[test]
    fn nested_ok() {
        assert!(c(&NestedBlockDepth::new(), "fun f() {}\n").is_empty());
    }
    #[test]
    fn nested_bad() {
        assert!(!c(&NestedBlockDepth { threshold: 2 }, "fun f(){{{}}}\n").is_empty());
    }
    #[test]
    fn large_class_ok() {
        assert!(c(&LargeClass::new(), "class C{}\n").is_empty());
    }
    #[test]
    fn complex_condition_ok() {
        assert!(c(&ComplexCondition::new(), "if (x)\n").is_empty());
    }
    #[test]
    fn complex_condition_bad() {
        assert!(!c(&ComplexCondition { threshold: 2 }, "if (x&&y||z&&w&&v)\n").is_empty());
    }

    // New complexity rules (8 additional, total 15/15)
    #[test]
    fn string_dup_ok() {
        assert!(c(
            &StringLiteralDuplication::new(),
            "fun f() { val a = \"x\" }\n"
        )
        .is_empty());
    }
    #[test]
    fn string_dup_bad() {
        assert!(!c(
            &StringLiteralDuplication::new(),
            "fun f() { val a = \"dup\"; val b = \"dup\"; val c = \"dup\" }\n"
        )
        .is_empty());
    }
    #[test]
    fn cognitive_ok() {
        assert!(c(&CognitiveComplexMethod::new(), "fun f() { }\n").is_empty());
    }
    #[test]
    fn cognitive_bad() {
        let mut s = String::from("fun f() {\n");
        for _ in 0..20 {
            s.push_str("if (x) { }\n");
        }
        s.push_str("}\n");
        assert!(!c(&CognitiveComplexMethod::new(), &s).is_empty());
    }
    #[test]
    fn overload_ok() {
        assert!(c(
            &MethodOverloading::new(),
            "class C { fun a() { } }\nfun b() { }\n"
        )
        .is_empty());
    }
    #[test]
    fn overload_bad() {
        assert!(!c(
            &MethodOverloading { threshold: 2 },
            "class C {\nfun a() { }\nfun a(x:Int) { }\nfun a(x:Long) { }\n}\n"
        )
        .is_empty());
    }
    #[test]
    fn named_ok() {
        assert!(c(&NamedArguments::new(), "fun f() { foo(x = 1, y = 2) }\n").is_empty());
    }
    #[test]
    fn named_bad() {
        assert!(!c(&NamedArguments::new(), "fun f() { foo(1, 2, 3, 4) }\n").is_empty());
    }
    #[test]
    fn spread_ok() {
        assert!(c(&SpreadOperator, "fun f() { listOf(1) }\n").is_empty());
    }
    #[test]
    fn spread_bad() {
        assert!(!c(&SpreadOperator, "fun f() { listOf(*args) }\n").is_empty());
    }
    #[test]
    fn complex_iface_ok() {
        assert!(c(&ComplexInterface::new(), "interface I { fun a() }\n").is_empty());
    }
    #[test]
    fn labeled_ok() {
        assert!(c(&LabeledExpression, "fun f() { for (x in 1..10) { } }\n").is_empty());
    }
    #[test]
    fn labeled_bad() {
        assert!(!c(
            &LabeledExpression,
            "fun f() { loop@ for (x in 1..10) { } }\n"
        )
        .is_empty());
    }
    #[test]
    fn safe_chain_bad() {
        assert!(!c(&ReplaceSafeCallChainWithRun, "fun f() { x?.let { } }\n").is_empty());
    }
}
