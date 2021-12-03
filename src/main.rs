use std::{
    fs::File,
    io::{BufReader, Read},
};

mod antlr;
mod java_test;

use java_test::parse_java;

fn main() {
    println!("Java Test:");
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
