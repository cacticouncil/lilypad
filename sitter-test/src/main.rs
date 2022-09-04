use std::{
    fs::File,
    io::{BufReader, Read},
};
use tree_sitter::{Language, Parser, Tree, TreeCursor};

/* ------- Parsing File ------- */
fn main() {
    // Get Java Language
    extern "C" {
        fn tree_sitter_python() -> Language;
    }
    let language = unsafe { tree_sitter_python() };

    // Create Parser
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();

    // Parse Test String
    let source = get_test_string("test.py");
    let tree = parser.parse(source, None).unwrap();
    println!("{}", make_tree_str(&tree));
}

fn get_test_string(name: &'static str) -> String {
    let file = File::open(name).expect("test file not found");
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader
        .read_to_string(&mut contents)
        .expect("could not read file");
    return contents;
}

/* ------- Displaying Tree ------- */
fn make_tree_str(tree: &Tree) -> String {
    make_branch(&mut tree.root_node().walk(), "", true)
}

fn make_branch(cursor: &mut TreeCursor, indent: &str, last: bool) -> String {
    let join_symbol = if last { "└─ " } else { "├─ " };
    let current_node = cursor.node();

    let mut result = String::new();
    result.push_str(indent);
    result.push_str(join_symbol);
    result.push_str(current_node.kind());
    result.push('\n');

    let child_count = current_node.child_count();
    if cursor.goto_first_child() {
        let mut child_idx = 1;
        loop {
            let new_indent = format!("{}{}", indent, if last { "    " } else { "│  " });
            let child_branch = make_branch(cursor, &new_indent, child_idx == child_count);

            result.push_str(&child_branch);
            child_idx += 1;

            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }

    result
}
