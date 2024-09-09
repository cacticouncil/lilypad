use ropey::Rope;
use tree_sitter::{InputEdit, Tree, TreeCursor};

use super::Language;

pub struct TreeManager {
    tree: Tree,
}

/* ------- Parsing  ------- */
impl TreeManager {
    /// create empty tree
    pub fn new(lang: &mut Language) -> TreeManager {
        TreeManager {
            tree: lang.parser.parse("", None).unwrap(),
        }
    }

    pub fn get_cursor(&self) -> TreeCursor {
        self.tree.walk()
    }

    pub fn replace(&mut self, source: &Rope, lang: &mut Language) {
        self.parse(source, false, lang);
    }

    pub fn update(&mut self, source: &Rope, change: InputEdit, lang: &mut Language) {
        self.tree.edit(&change);
        self.parse(source, true, lang);
    }

    fn parse(&mut self, source: &Rope, use_old: bool, lang: &mut Language) {
        self.tree = lang
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
