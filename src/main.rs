#![feature(try_blocks)]
#![feature(coerce_unsized)]

use std::{
    fs::File,
    io::{BufReader, Read},
};

mod antlr {
    pub mod csvlexer;
    pub mod csvlistener;
    pub mod csvparser;
    pub mod csvvisitor;

    pub mod javalexer;
    pub mod javalistener;
    pub mod javaparser;
}
mod csv_test;
mod java_test;

use csv_test::parse_csv;
use java_test::parse_java;

fn main() {
    println!("CSV Test:");
    let csv_input = get_test_string("test.csv");
    parse_csv(&csv_input);
    println!("----------");
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
