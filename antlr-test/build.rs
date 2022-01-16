use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Command;

fn main() {
    let _ = gen_for_grammar("JavaLexer.g4", "JavaParser.g4");
    fix_java_parser();
    println!("cargo:rerun-if-changed=build.rs");
}

fn gen_for_grammar(lexer: &str, parser: &str) -> Result<(), Box<dyn Error>> {
    // antlr path
    let antlr_name = "antlr4-4.9.4-rust-0.3.jar";
    let antlr_dir = env::current_dir()
        .unwrap()
        .join(antlr_name);
    let antlr_path = antlr_dir 
        .as_os_str()
        .to_str()
        .unwrap();
    
    // grammars path
    let grammars_dir = env::current_dir().unwrap().join("grammars");

    // run command
    let _o = Command::new("java")
        .current_dir(grammars_dir)
        .arg("-cp")
        .arg(antlr_path)
        .arg("org.antlr.v4.Tool")
        .arg("-Dlanguage=Rust")
        .arg("-o")
        .arg("../src/antlr")
        .arg("-listener")
        .arg("-visitor")
        // .arg("-lib")
        .arg(lexer)
        .arg(parser)
        .spawn()
        .expect("antlr tool failed to start")
        .wait_with_output()?;

    // if grammar files change, regenerate
    println!("cargo:rerun-if-changed=grammars/{}", lexer);
    println!("cargo:rerun-if-changed=grammars/{}", parser);

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

    // Recreate the file and dump the processed contents to it
    let mut dst = File::create(&file_path).expect("could recreate java parser");
    dst.write(data.as_bytes()).expect("could not write");
}
