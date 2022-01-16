mod antlr;

use std::{
    ops::Deref,
    fs::File,
    io::{BufReader, Read},
};

use antlr_rust::common_token_stream::CommonTokenStream;
use antlr_rust::input_stream::InputStream;
use antlr_rust::recognizer::Recognizer;
use antlr_rust::token_factory::CommonTokenFactory;
use antlr_rust::tree::{ParseTree, Tree};

use antlr::javalexer::JavaLexer;
use antlr::javaparser::*;

fn main() {
    let java_input = get_test_string("test.java");
    parse_java(&java_input);
}

fn get_test_string(name: &'static str) -> String {
    let file = File::open(name).expect("test file not found");
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader
        .read_to_string(&mut contents)
        .expect("could not read file");
    return contents;
}

pub fn parse_java(input: &str) {
    let lexer = JavaLexer::new(InputStream::new(&*input));
    let token_source = CommonTokenStream::new(lexer);
    let mut parser = JavaParser::new(token_source);
    let tree = parser.compilationUnit().unwrap();
    let tree_string = pretty_tree(
        &*tree as &dyn ParseTree<TF = CommonTokenFactory, Ctx = JavaParserContextType>,
        &*parser.get_rule_names(),
    );
    println!("{}", tree_string);
}

/* ------- Displaying as a Tree ------- */
fn pretty_tree<'a, T: Tree<'a> + ?Sized>(tree: &T, rule_names: &[&str]) -> String {
    make_branch(tree, rule_names, "", true)
}

fn make_branch<'a, T: Tree<'a> + ?Sized>(
    tree: &T,
    rule_names: &[&str],
    indent: &str,
    last: bool,
) -> String {
    let new_indent = format!("{}{}", indent, if last { "    " } else { "│  " });
    let join_symbol = (if last { "└─ " } else { "├─ " }).to_string();

    let name = escape_special_chars(&tree.get_node_text(rule_names));
    if tree.get_child_count() == 0 {
        return format!("{}{}{}\n", indent, join_symbol, name);
    }

    let mut result = String::new();
    result.extend(indent.chars());
    result.extend(join_symbol.chars());
    result.extend(name.chars());
    result.push('\n');
    let children = tree.get_children();
    result = children.identify_last()
        .map(|(child , last)| {
            make_branch(
                child.deref(),
                rule_names,
                &new_indent,
                last
            )
        })
        .fold(result, |mut acc, text| {
            acc.extend(text.chars());
            acc
        });
    result
}

fn escape_special_chars(data: &str) -> String {
    let mut res = String::with_capacity(data.len());
    data.chars().for_each(|ch| match ch {
        '\t' => res.extend("\\t".chars()),
        '\n' => res.extend("\\n".chars()),
        '\r' => res.extend("\\r".chars()),
        _ => res.push(ch),
    });
    res
}

/* ------- Identify Last in Iterator ------- */
use std::iter;

pub trait IdentifyLast: Iterator + Sized {
    fn identify_last(self) -> Iter<Self>;
}

impl<I> IdentifyLast for I where I: Iterator {
    fn identify_last(self) -> Iter<Self> {
        Iter(true, self.peekable())
    }
}

pub struct Iter<I>(bool, iter::Peekable<I>) where I: Iterator;

impl<I> Iterator for Iter<I> where I: Iterator {
    type Item = (I::Item, bool);

    fn next(&mut self) -> Option<Self::Item> {
        self.1.next().map(|e| (e, self.1.peek().is_none()))
    }
}
