# Lilypad

A next generation dual modal code editor

## Prerequisite Installs

- Both
    - [Rust](https://rustup.rs/)
- Default Native Fonts
    - macOS: [SF Mono](https://developer.apple.com/fonts/)
    - Windows: [Roboto Mono Font](https://fonts.google.com/specimen/Roboto+Mono)
- Web
    - [wasm-pack](https://rustwasm.github.io/wasm-pack/)
    - LLVM (`tree-sitter-python-wasm-compatible` requires `llvm-ar`) 
    - [Just Command Runner](https://github.com/casey/just) (optional)
    - [Host These Things Please](https://crates.io/crates/https) (optional)

## Running

### Native App

1. `cargo run`

### VSCode Extension

1. `just wasm-vscode`
2. Open `lilypad-vscode/` in [VSCode Insiders](https://code.visualstudio.com/insiders/)
3. `npm install`
4. Run using VSCode

### In Browser

1. `just wasm-web`
2. `cd lilypad-web`
3. `http`
