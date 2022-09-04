use std::path::Path;

fn main() {
    slint_build::compile("ui/test.slint").unwrap();
    compile_grammar();
}

fn compile_grammar() {
    let src_dir = Path::new("../tree-sitter-python/src");

    // compile parser
    let parser_path = src_dir.join("parser.c");
    cc::Build::new()
        .include(&src_dir)
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .file(&parser_path)
        .compile("parser");

    // compile scanner
    let scanner_path = src_dir.join("scanner.cc");
    cc::Build::new()
        .cpp(true)
        .include(&src_dir)
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .file(&scanner_path)
        .compile("scanner");

    // rebuild when either of those files are updated
    println!("cargo:rerun-if-changed={}", parser_path.to_str().unwrap());
    println!("cargo:rerun-if-changed={}", scanner_path.to_str().unwrap());
}
