# Lilypad

A next generation dual modal code editor

## Prerequisite Installs

- Both
    - [Rust](https://rustup.rs/)
    - [Roboto Mono Font](https://fonts.google.com/specimen/Roboto+Mono)
- Web
    - [wasm-pack](https://rustwasm.github.io/wasm-pack/)
    - [Just Command Runner](https://github.com/casey/just) (optional)
    - [Host These Things Please](https://crates.io/crates/https) (optional)

## Running

### Native App

1. `cargo run`

### VSCode Extension

1. `just wasm-vscode`
2. Open `lilypad-vscode/` in [VSCode Insiders](https://code.visualstudio.com/insiders/)
3. Run using VSCode

### In Browser

1. `just wasm-web`
2. `http`
