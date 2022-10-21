use tree_sitter::{Language, Parser, Tree, TreeCursor};

pub struct TreeManager {
    pub tree: Tree,
    parser: Parser,
}

/* ------- Parsing  ------- */
impl TreeManager {
    pub fn new(source: &str) -> TreeManager {
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
        let tree = parser.parse(source, None).unwrap();

        TreeManager { tree, parser }
    }

    pub fn replace_tree(&mut self, new_source: &str) {
        self.tree = self.parser.parse(new_source, None).unwrap();
    }

    #[allow(dead_code)]
    pub fn update_tree(&mut self) {
        todo!()
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
