use regex::Regex;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;

fn main() {
    let antlr_path = "../antlr4-4.8-2-SNAPSHOT-complete.jar";

    let _ = gen_for_grammar("JavaLexer.g4 JavaParser.g4", antlr_path);

    // fix the java parser
    fix_java_parser();

    println!("cargo:rerun-if-changed=build.rs");
}

fn gen_for_grammar(grammar_file_name: &str, antlr_path: &str) -> Result<(), Box<dyn Error>> {
    let input = env::current_dir().unwrap().join("grammars");
    let file_name = grammar_file_name.to_owned() + ".g4";

    let _c = Command::new("java")
        .current_dir(input)
        .arg("-cp")
        .arg(antlr_path)
        .arg("org.antlr.v4.Tool")
        .arg("-Dlanguage=Rust")
        .arg("-o")
        .arg("../src/antlr")
        .arg(&file_name)
        .spawn()
        .expect("antlr tool failed to start")
        .wait_with_output()?;

    println!("cargo:rerun-if-changed=grammars/{}", file_name);
    Ok(())
}

fn fix_java_parser() {
    // Open and read the file entirely
    let file_path = env::current_dir().unwrap().join("src/antlr/javaparser.rs");
    let mut src = File::open(&file_path).expect("could not find java parser");
    let mut data = String::new();
    src.read_to_string(&mut data)
        .expect("could not read java parser");
    drop(src); // Close the file early

    // keywords can still be used in names if they are raw identifiers
    data = data
        .replace(" type(", " r#type(")
        .replace(".type(", ".r#type(");

    // remove brackets on one-liners
    data = Regex::new("([if|while]) \\{ (.*) \\} \\{")
        .unwrap()
        .replace_all(&data, "$1 $2 {")
        .to_string();

    // Recreate the file and dump the processed contents to it
    let mut dst = File::create(&file_path).expect("could recreate java parser");
    dst.write(data.as_bytes()).expect("could not write");
}
