use crate::block_editor::BlockType;

pub struct LanguageConfig {
    /// Name of the language. Used as an ID and potentially for UI
    pub name: &'static str,

    /// Tree-sitter language
    ts_lang: fn() -> tree_sitter_c2rust::Language,

    /// Tree-sitter highlight query
    pub highlight_query: &'static str,

    /// The character that starts a new scope (so should increase the indent)
    pub new_scope_char: NewScopeChar,

    /// Assigns a node a block type to draw
    node_categorizer: fn(&tree_sitter_c2rust::Node) -> Option<BlockType>,

    /// The IDs for a string, and the start and end. Used for pseudo-selections
    pub string_node_ids: StringNodeIDs,

    /// Snippets to use for the palette. Must end with a newline.
    pub palette: &'static [Snippet],
}

#[derive(PartialEq, Clone, Copy)]
pub enum NewScopeChar {
    Colon,
    Brace,
}

impl NewScopeChar {
    pub const fn char(&self) -> char {
        match self {
            NewScopeChar::Colon => ':',
            NewScopeChar::Brace => '{',
        }
    }
}

#[derive(Clone, Copy)]
pub struct StringNodeIDs {
    pub string: u16,
    pub string_bounds: &'static [u16],
}

pub struct Snippet {
    pub id: &'static str,
    pub source: &'static str,
}

impl Snippet {
    pub const fn new(id: &'static str, source: &'static str) -> Snippet {
        Snippet { id, source }
    }
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
    new_scope_char: NewScopeChar::Colon,
    node_categorizer: |node| {
        use BlockType::*;

        match node.kind() {
            // scopes
            "class_definition" => Some(Object),
            "function_definition" => Some(FunctionDef),
            "while_statement" => Some(While),
            "if_statement" => Some(If),
            "for_statement" => Some(For),
            "try_statement" => Some(Try),

            // normal expressions (incomplete)
            "import_statement" => Some(Generic),
            "expression_statement" => Some(Generic),
            "continue_statement" => Some(Generic),
            "break_statement" => Some(Generic),
            "pass_statement" => Some(Generic),

            // comments
            "comment" => Some(Comment),

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
    palette: &[
        Snippet::new(
            "if",
            "if condition:\n    pass\nelif condition:\n    pass\nelse:\n    pass\n",
        ),
        Snippet::new("while", "while condition:\n    pass\n"),
        Snippet::new(
            "class",
            "class Class:\n    def __init__(self):\n        pass\n",
        ),
        Snippet::new("func", "def function():\n    pass\n"),
        Snippet::new(
            "try",
            "try:\n    pass\nexcept:\n    pass\nelse:\n    pass\nfinally:\n    pass\n",
        ),
    ],
};

const JAVA_LANGUAGE: LanguageConfig = LanguageConfig {
    name: "java",
    ts_lang: tree_sitter_java::language,
    highlight_query: tree_sitter_java::HIGHLIGHT_QUERY,
    new_scope_char: NewScopeChar::Brace,
    node_categorizer: |node| {
        use BlockType::*;

        match node.kind() {
            // scopes
            "class_declaration" => Some(Object),
            "interface_declaration" => Some(Object),
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

            // comments
            "line_comment" => Some(Comment),
            "block_comment" => Some(Comment),

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
    palette: &[
        Snippet::new(
            "if",
            "if (condition) {\n    \n} else if (condition) {\n    \n} else {\n    \n}\n",
        ),
        Snippet::new(
            "class",
            "public class MyClass {\n    public MyClass() {\n        \n    }\n}\n",
        ),
        Snippet::new("while", "while (condition) {\n    \n}\n"),
        Snippet::new("method", "public void myMethod() {\n    \n}\n"),
        Snippet::new(
            "try",
            "try {\n    \n} catch (Exception e) {\n    \n} finally {\n    \n}\n",
        ),
    ],
};

const CS_LANGUAGE: LanguageConfig = LanguageConfig {
    name: "c#",
    ts_lang: tree_sitter_c_sharp::language,
    highlight_query: tree_sitter_c_sharp::HIGHLIGHT_QUERY,
    new_scope_char: NewScopeChar::Brace,
    node_categorizer: |node| {
        use BlockType::*;

        match node.kind() {
            // scopes
            "class_declaration" => Some(Object),
            "method_declaration" => Some(FunctionDef),
            "while_statement" => Some(While),
            "if_statement" => {
                if node.prev_sibling().map_or("", |s| s.kind()) == "else" {
                    None
                } else {
                    Some(If)
                }
            }
            "for_statement" => Some(For),
            "try_statement" => Some(Try),
            "switch_statement" => Some(Switch),
            "case_switch_label" => Some(Divider),
            "default_switch_label" => Some(Divider),
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
            "break_statement" => Some(Generic),
            "return_statement" => Some(Generic),
            "assert_statement" => Some(Generic),

            // comments
            "line_comment" => Some(Comment),
            "block_comment" => Some(Comment),

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
    palette: &[
        Snippet::new(
            "if",
            "if (condition) {\n    \n} else if (condition) {\n    \n} else {\n    \n}\n",
        ),
        Snippet::new(
            "class",
            "public class MyClass {\n    public MyClass() {\n        \n    }\n}\n",
        ),
        Snippet::new("while", "while (condition) {\n    \n}\n"),
        Snippet::new("func", "public void myFunction() {\n    \n}\n"),
        Snippet::new(
            "try",
            "try {\n    \n} catch (Exception e) {\n    \n} finally {\n    \n}\n",
        ),
    ],
};
