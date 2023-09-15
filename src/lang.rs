use crate::block_editor::BlockType;

pub struct LanguageConfig {
    /// Name of the language. Used as an ID and potentially for UI
    pub name: &'static str,

    /// Tree-sitter language
    ts_lang: fn() -> tree_sitter_c2rust::Language,

    /// Tree-sitter highlight query
    pub highlight_query: &'static str,

    /// The character that starts a new scope (so should increase the indent)
    pub new_scope_char: char,

    /// Assigns a node a block type to draw
    node_categorizer: fn(&tree_sitter_c2rust::Node) -> Option<BlockType>,

    /// The IDs for a string, and the start and end. Used for pseudo-selections
    pub string_node_ids: StringNodeIDs,
}

#[derive(Clone, Copy)]
pub struct StringNodeIDs {
    pub string: u16,
    pub string_bounds: &'static [u16],
}

impl LanguageConfig {
    pub fn tree_sitter(&self) -> tree_sitter_c2rust::Language {
        (self.ts_lang)()
    }

    pub fn categorize_node(&self, node: &tree_sitter_c2rust::Node) -> Option<BlockType> {
        (self.node_categorizer)(node)
    }
}

pub fn lang_for_file(file_name: &str) -> &'static LanguageConfig {
    match file_name.split('.').last() {
        Some("py") => &PYTHON_LANGUAGE,
        Some("java") => &JAVA_LANGUAGE,
        Some("cs") => &CS_LANGUAGE,
        _ => &PYTHON_LANGUAGE, // TODO: plain text mode?
    }
}

const PYTHON_LANGUAGE: LanguageConfig = LanguageConfig {
    name: "python",
    ts_lang: tree_sitter_python::language,
    highlight_query: tree_sitter_python::HIGHLIGHT_QUERY,
    new_scope_char: ':',
    node_categorizer: |node| {
        use BlockType::*;

        match node.kind() {
            // scopes
            "class_definition" => Some(Class),
            "function_definition" => Some(FunctionDef),
            "while_statement" => Some(While),
            "if_statement" => Some(If),
            "for_statement" => Some(For),
            "try_statement" => Some(Try),

            // normal expressions (incomplete)
            "import_statement" => Some(Generic),
            "expression_statement" => Some(Generic),
            "comment" => Some(Generic),

            // dividers to keep generics from merging
            "else_clause" => Some(Divider),
            "elif_clause" => Some(Divider),
            "except_clause" => Some(Divider),

            // do not handle the rest
            _ => None,
        }
    },
    string_node_ids: StringNodeIDs {
        string: 230,
        string_bounds: &[104, 107], // 104 is string start, 107 is string end
    },
};

const JAVA_LANGUAGE: LanguageConfig = LanguageConfig {
    name: "java",
    ts_lang: tree_sitter_java::language,
    highlight_query: tree_sitter_java::HIGHLIGHT_QUERY,
    new_scope_char: '{',
    node_categorizer: |node| {
        use BlockType::*;

        match node.kind() {
            // scopes
            "class_declaration" => Some(Class),
            "method_declaration" => Some(FunctionDef),
            "while_statement" => Some(While),
            "if_statement" => {
                // the java grammar treats else if as else, if_statement
                // so check that is isn't that
                if node.prev_sibling().map_or("", |s| s.kind()) == "else" {
                    None
                } else {
                    Some(If)
                }
            }
            "for_statement" => Some(For),
            "try_statement" => Some(Try),

            // normal expressions (incomplete)
            "import_declaration" => Some(Generic),
            "expression_statement" => Some(Generic),
            "local_variable_declaration" => {
                // don't create a block for a for loop's variable declaration
                if node.parent().map_or("", |p| p.kind()) == "for_statement" {
                    None
                } else {
                    Some(Generic)
                }
            }
            "field_declaration" => Some(Generic),
            "return_statement" => Some(Generic),
            "assert_statement" => Some(Generic),
            "line_comment" => Some(Generic),
            "block_comment" => Some(Generic),

            // dividers to keep generics from merging
            "block" => Some(Divider),

            // do not handle the rest
            _ => None,
        }
    },
    string_node_ids: StringNodeIDs {
        string: 141,
        string_bounds: &[11, 12], // 11 is single quote, 12 is double quote
    },
};

const CS_LANGUAGE: LanguageConfig = LanguageConfig {
    name: "c#",
    ts_lang: tree_sitter_c_sharp::language,
    highlight_query: tree_sitter_c_sharp::HIGHLIGHT_QUERY,
    new_scope_char: '{',
    node_categorizer: |node| {
        use BlockType::*;

        match node.kind() {
            // scopes
            "class_declaration" => Some(Class),
            "method_declaration" => Some(FunctionDef),
            "while_statement" => Some(While),
            "if_statement" => Some(If),//{
                // the java grammar treats else if as else, if_statement
                // so check that is isn't that
                //if node.prev_sibling().map_or("", |s| s.kind()) == "else" {
                  //  None
              //  } else {
               //     Some(If)
               // }
            //}
            "for_statement" => Some(For),
            "try_statement" => Some(Try),

            // normal expressions (incomplete)
            "import_declaration" => Some(Generic),
            "expression_statement" => Some(Generic),
            "local_variable_declaration" => {
                // don't create a block for a for loop's variable declaration
                if node.parent().map_or("", |p| p.kind()) == "for_statement" {
                    None
                } else {
                    Some(Generic)
                }
            }
            "field_declaration" => Some(Generic),
            "return_statement" => Some(Generic),
            "assert_statement" => Some(Generic),
            "line_comment" => Some(Generic),
            "block_comment" => Some(Generic),

            // dividers to keep generics from merging
            "block" => Some(Divider),

            // do not handle the rest
            _ => None,
        }
    },
    string_node_ids: StringNodeIDs {
        string: 141,
        string_bounds: &[11, 12], // 11 is single quote, 12 is double quote
    },
};