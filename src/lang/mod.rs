pub mod config;
pub mod highlighter;
pub mod tree_manager;

use std::cell::RefCell;

use config::LanguageConfig;

pub struct Language {
    pub config: &'static config::LanguageConfig,
    _ts_language: tree_sitter::Language,
    pub parser: tree_sitter::Parser,
    pub highlighter: RefCell<highlighter::Highlighter>, // use ref cells because you can only have one mutable reference to a property of a struct at a time
    pub highlight_config: RefCell<highlighter::HighlightConfiguration>,
}

impl Language {
    pub fn for_file(file_name: &str) -> Self {
        Self::new(LanguageConfig::for_file(file_name))
    }

    fn new(config: &'static config::LanguageConfig) -> Self {
        let ts_language = config.tree_sitter();

        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&ts_language).unwrap();

        let mut highlight_config = highlighter::HighlightConfiguration::new(
            config.tree_sitter(),
            config.name,
            config.highlight_query,
            "",
            "",
        )
        .unwrap();
        highlight_config.configure(&config.highlight.iter().map(|x| x.0).collect::<Vec<&str>>());

        let highlighter = highlighter::Highlighter::new();
        Self {
            config,
            _ts_language: ts_language,
            parser,
            highlighter: RefCell::new(highlighter),
            highlight_config: RefCell::new(highlight_config),
        }
    }
}
