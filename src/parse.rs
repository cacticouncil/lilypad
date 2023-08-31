use ropey::Rope;
use tree_sitter_c2rust::{InputEdit, Parser, Tree, TreeCursor};

use crate::lang::LanguageConfig;

pub struct TreeManager {
    tree: Tree,
    parser: Parser,
}

/* ------- Parsing  ------- */
impl TreeManager {
    /// create empty tree
    pub fn new(lang: &LanguageConfig) -> TreeManager {
        // Create Parser
        let mut parser = Parser::new();
        parser.set_language(lang.tree_sitter()).unwrap();

        // Parse initial source
        let tree = parser.parse("", None).unwrap();

        TreeManager { tree, parser }
    }

    pub fn change_language(&mut self, lang: &LanguageConfig) {
        self.parser.set_language(lang.tree_sitter()).unwrap();
    }

    pub fn get_cursor(&self) -> TreeCursor {
        self.tree.walk()
    }

    pub fn replace(&mut self, source: &Rope) {
        self.parse(source, false);
    }

    pub fn update(&mut self, source: &Rope, change: InputEdit) {
        self.tree.edit(&change);
        self.parse(source, true);
    }

    fn parse(&mut self, source: &Rope, use_old: bool) {
        self.tree = self
            .parser
            .parse_with(
                &mut |byte, _| {
                    if byte <= source.len_bytes() {
                        let (chunk, start_byte, _, _) = source.chunk_at_byte(byte);
                        chunk[byte - start_byte..].as_bytes()
                    } else {
                        // out of range
                        &[]
                    }
                },
                if use_old { Some(&self.tree) } else { None },
            )
            .unwrap();
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

        let mut result = format!(
            "{}{}{} ({})\n",
            indent,
            join_symbol,
            current_node.kind(),
            current_node.kind_id()
        );

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
