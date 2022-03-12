# Rust Parser Tests

Small implementations of various parsing libraries in Rust

## Antlr Test

`build.rs` will generate the appropriate files from the antlr grammar given `antlr4-4.9.4-rust-0.3.jar` is in the root directory. Obtain that file by:

1. Clone https://github.com/rrevenantt/antlr4/tree/rust-target
2. Update the submodule `antlr4/runtime/Rust` to the latest version of https://github.com/rrevenantt/antlr4rust
3. Build by running

```
export MAVEN_OPTS="-Xmx1G"
mvn clean
mvn -DskipTests install
```

## Sitter Test

The Java grammar is stored as a submodule so to clone you need to run `git clone --recurse-submodules`

## Sitter + Iced

Displays the result of Tree Sitter using Iced

The Python grammar is stored as a submodule so to clone you need to run `git clone --recurse-submodules`
