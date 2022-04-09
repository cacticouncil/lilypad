use tree_sitter::{Language, Parser, Tree};

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
