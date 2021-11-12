use antlr_rust::common_token_stream::CommonTokenStream;
use antlr_rust::input_stream::InputStream;
use antlr_rust::tree::ParseTree;

use crate::antlr::javalexer::JavaLexer;
use crate::antlr::javaparser::*;

pub fn parse_java(input: &str) {
    let lexer = JavaLexer::new(InputStream::new(&*input));
    let token_source = CommonTokenStream::new(lexer);
    let mut parser = JavaParser::new(token_source);
    let result = parser.compilationUnit().unwrap();
    println!("{}", result.to_string_tree(&*parser));
}
