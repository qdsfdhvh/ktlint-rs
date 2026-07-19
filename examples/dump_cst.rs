fn main() {
    let src = std::fs::read_to_string("/tmp/test_class.kt").unwrap();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_kotlin_sg::LANGUAGE.into()).unwrap();
    let tree = parser.parse(&src, None).unwrap();
    print_node(tree.root_node(), src.as_bytes(), 0);
}
fn print_node(n: tree_sitter::Node, b: &[u8], d: usize) {
    let text = if n.child_count() == 0 { format!(" {:?}", n.utf8_text(b).unwrap_or("")) } else { String::new() };
    println!("{}{}{}", "  ".repeat(d), n.kind(), text);
    for i in 0..n.child_count() {
        if let Some(c) = n.child(i) { print_node(c, b, d + 1); }
    }
}
