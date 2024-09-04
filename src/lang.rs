use crate::block_editor::BlockType;

pub struct LanguageConfig {
    /// Name of the language. Used as an ID and potentially for UI
    pub name: &'static str,

    /// Tree-sitter language
    ts_lang: tree_sitter_language::LanguageFn,

    /// Tree-sitter highlight query
    pub highlight_query: &'static str,

    /// The character that starts a new scope (so should increase the indent)
    pub new_scope_char: NewScopeChar,

    /// Assigns a node a block type to draw
    node_categorizer: fn(&tree_sitter::Node) -> Option<BlockType>,

    /// The IDs for a string, and the start and end. Used for pseudo-selections
    pub string_node_ids: StringNodeIDs,

    /// Snippets to use for the palette. Must end with a newline.
    pub palettes: &'static [Palette],
}

pub struct Palette {
    pub name: &'static str,
    pub snippets: &'static [Snippet],
}

impl Palette {
    pub const fn new(name: &'static str, snippets: &'static [Snippet]) -> Palette {
        Palette { name, snippets }
    }
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
    pub fn tree_sitter(&self) -> tree_sitter::Language {
        tree_sitter::Language::new(self.ts_lang)
    }

    pub fn categorize_node(&self, node: &tree_sitter::Node) -> Option<BlockType> {
        (self.node_categorizer)(node)
    }
}

pub fn lang_for_file(file_name: &str) -> &'static LanguageConfig {
    match file_name.split('.').last() {
        Some("py") => &PYTHON_LANGUAGE,
        Some("java") => &JAVA_LANGUAGE,
        // Some("cpp") | Some("h") | Some("hpp") => &CPP_LANGUAGE,
        Some("cs") => &CS_LANGUAGE,
        _ => &PYTHON_LANGUAGE, // TODO: plain text mode?
    }
}

const PYTHON_LANGUAGE: LanguageConfig = LanguageConfig {
    name: "python",
    ts_lang: tree_sitter_python::LANGUAGE,
    highlight_query: tree_sitter_python::HIGHLIGHTS_QUERY,
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

            // normal expressions
            // TODO: check exhastiveness
            "import_statement" => Some(Generic),
            "import_from_statement" => Some(Generic),
            "expression_statement" => Some(Generic),
            "continue_statement" => Some(Generic),
            "break_statement" => Some(Generic),
            "pass_statement" => Some(Generic),
            "return_statement" => Some(Generic),

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
    palettes: &[
        Palette::new(
            "General",
            &[
                Snippet::new("import_module", "import module\n"),
                Snippet::new("import_from", "from module import thing\n"),
                Snippet::new("import_as", "import module as name\n"),
                Snippet::new("var_assign", "val = 0\n"),
                Snippet::new("var_assign_string", "val = \"Hello world\"\n"),
                Snippet::new("var_assign_list", "val = [1, 2, 3]\n"),
                Snippet::new("var_assign_dict", "val = {'a': 1, 'b': 'tw0'}\n"),
                Snippet::new("var_assign_tuple", "val = (False, 1, 2.0, '3')\n"),
                Snippet::new("var_assign_set", "val = {1, 2, 3}\n"),
            ],
        ),
        Palette::new(
            "Classes",
            &[
                Snippet::new(
                    "class_declaration",
                    "class ClassName:\n    def __init__(self, param):\n        pass\n",
                ),
                Snippet::new("instance_method", "def method(self, param):\n    pass\n"),
                Snippet::new(
                    "static_method",
                    "@staticmethod\ndef method(param):\n    pass\n",
                ),
                Snippet::new("class_instance", "instance = ClassName()\n"),
            ],
        ),
        Palette::new(
            "Control",
            &[
                Snippet::new("for", "for item in range(0, 10):\n    pass\n"),
                Snippet::new("while", "while 0 == 0:\n    pass\n"),
                Snippet::new("break", "break\n"),
                Snippet::new("continue", "continue\n"),
                Snippet::new("if", "if 0 < 0:\n    pass\n"),
                Snippet::new("if_else", "if 0 < 0:\n    pass\nelse:\n    pass\n"),
                Snippet::new(
                    "if_elif_else",
                    "if 0 < 0:\n    pass\nelif 0 > 0:\n    pass\nelse:\n    pass\n",
                ),
                Snippet::new(
                    "try",
                    "try:\n    pass\nexcept:\n    pass\nelse:\n    pass\nfinally:\n    pass\n",
                ),
            ],
        ),
        Palette::new(
            "Functions",
            &[
                Snippet::new("function_def", "def function(args):\n    return\n"),
                Snippet::new("function_call", "function(args) \n"),
                Snippet::new("return_val", "return value\n"),
                Snippet::new("return", "return\n"),
            ],
        ),
        Palette::new(
            "Logic",
            &[
                Snippet::new("equals", "a == b\n"),
                Snippet::new("not_equals", "a != b\n"),
                Snippet::new("greater_than", "a > b\n"),
                Snippet::new("less_than", "a < b\n"),
                Snippet::new("greater_than_or_equal", "a >= b\n"),
                Snippet::new("less_than_or_equal", "a <= b\n"),
                Snippet::new("and", "a and b\n"),
                Snippet::new("or", "a or b\n"),
                Snippet::new("not", "not a\n"),
                Snippet::new("in", "a in b\n"),
                Snippet::new("is", "a is b\n"),
            ],
        ),
        Palette::new(
            "Arithmetic",
            &[
                Snippet::new("add", "a + b\n"),
                Snippet::new("subtract", "a - b\n"),
                Snippet::new("multiply", "a * b\n"),
                Snippet::new("divide", "a / b\n"),
                Snippet::new("modulo", "a % b\n"),
                Snippet::new("exponent", "a ** b\n"),
                Snippet::new("floor_divide", "a // b\n"),
            ],
        ),
    ],
};

const JAVA_LANGUAGE: LanguageConfig = LanguageConfig {
    name: "java",
    ts_lang: tree_sitter_java::LANGUAGE,
    highlight_query: tree_sitter_java::HIGHLIGHTS_QUERY,
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
    palettes: &[Palette::new(
        "General",
        &[
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
    )],
};

const CS_LANGUAGE: LanguageConfig = LanguageConfig {
    name: "c#",
    ts_lang: tree_sitter_c_sharp::LANGUAGE,
    highlight_query: tree_sitter_c_sharp::HIGHLIGHTS_QUERY,
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
    palettes: &[Palette::new(
        "General",
        &[
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
    )],
};

/*
const CPP_LANGUAGE: LanguageConfig = LanguageConfig {
    name: "cpp",
    ts_lang: tree_sitter_cpp::language,
    highlight_query: tree_sitter_cpp::HIGHLIGHT_QUERY,
    new_scope_char: NewScopeChar::Brace,
    node_categorizer: |node| {
        use BlockType::*;

        match node.kind() {
            // scopes
            "class_specifier" => Some(Object),
            "struct_specifier" => Some(Object),
            "abstract_function_declarator" => Some(Object),
            "function_definition" => {
                // create one box around a template function
                if node.parent().map_or("", |s| s.kind()) == "template_declaration" {
                    None
                } else {
                    Some(FunctionDef)
                }
            }
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
            "template_declaration" => Some(FunctionDef),

            // normal expressions (incomplete)
            "preproc_include" => Some(Generic),
            "expression_statement" => Some(Generic),
            "continue_statement" => Some(Generic),
            "break_statement" => Some(Generic),
            "pass_statement" => Some(Generic),
            "local_variable_declaration" => {
                // don't create a block for a for loop's variable declaration
                if node.parent().map_or("", |p| p.kind()) == "for_statement" {
                    None
                } else {
                    Some(Generic)
                }
            }

            // comments
            "comment" => Some(Comment),

            // dividers to keep generics from merging
            "else_clause" => Some(Divider),
            "except_clause" => Some(Divider),

            // do not handle the rest
            _ => None,
        }
    },
    string_node_ids: StringNodeIDs {
        string: 360,
        string_bounds: &[162],
    },
    palette: &[],
};
*/
