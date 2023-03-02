use tree_sitter_c2rust::{InputEdit, Parser, Tree, TreeCursor};

pub struct TreeManager {
    tree: Tree,
    parser: Parser,
}

/* ------- Parsing  ------- */
impl TreeManager {
    pub fn new(source: &str) -> TreeManager {
        // Create Parser
        let mut parser = Parser::new();
        let language = tree_sitter_python_wasm_compatible::language();
        parser.set_language(language).unwrap();

        // Parse initial source
        let tree = parser.parse(source, None).unwrap();

        TreeManager { tree, parser }
    }

    pub fn get_cursor(&self) -> TreeCursor {
        self.tree.walk()
    }

    pub fn replace(&mut self, new_source: &str) {
        self.tree = self.parser.parse(new_source, None).unwrap();
    }

    pub fn update(&mut self, new_source: &str, change: InputEdit) {
        self.tree.edit(&change);
        self.tree = self.parser.parse(new_source, Some(&self.tree)).unwrap();
    }
}

/* ------- Displaying Tree  ------- */
#[allow(dead_code)]
impl TreeManager {
    pub fn make_tree_str(&self) -> String {
        let mut cursor = self.tree.root_node().walk();
        Self::make_branch(&mut cursor, "", true)
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
            let new_indent = format!("{}{}", indent, if last { "    " } else { "│  " });
            loop {
                let child_branch = Self::make_branch(cursor, &new_indent, child_idx == child_count);

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
}
