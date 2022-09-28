use tree_sitter::{Language, Parser, Tree, TreeCursor};

pub fn parse(source: &str) -> Tree {
    // Get Language
    extern "C" {
        fn tree_sitter_python() -> Language;
    }
    let language = unsafe { tree_sitter_python() };

    // Create Parser
    // In an actual application this wouldn't be built every time
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();

    // Parse Source
    parser.parse(source, None).unwrap()
}

/* ------- Displaying Tree (still here for debugging) ------- */
#[allow(dead_code)]
pub fn make_tree_str(tree: &Tree) -> String {
    make_branch(&mut tree.root_node().walk(), "", true)
}

#[allow(dead_code)]
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
        let new_indent = format!("{}{}", indent, if last { "    " } else { "│  " });
        loop {
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
