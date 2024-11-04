# Lilypad

A text-based visual code editor architected to supported multiple languages and platforms.

## Using It

- [Web](https://lilypad.cacticouncil.org/lilypad.html)
- [VSCode Extension](https://marketplace.visualstudio.com/items?itemName=CactiCouncil.lilypad-vscode)
- Native App To Be Released

## Development

### Prerequisite Installs

- Both
  - [Rust](https://rustup.rs/)
- Default Native Fonts
  - macOS: [SF Mono](https://developer.apple.com/fonts/)
  - Windows: [Roboto Mono](https://fonts.google.com/specimen/Roboto+Mono)
- Web
  - [wasm-pack](https://rustwasm.github.io/wasm-pack/)
  - LLVM (using the language grammars with wasm requires `llvm-ar`, add path to zhsprofile in order to build project)
  - [Just Command Runner](https://github.com/casey/just)
  - [Host These Things Please](https://crates.io/crates/https) (optional)

### Running

#### Native App

1. `cargo run`

#### VSCode Extension

1. `just wasm-vscode`
2. Open `lilypad-vscode/` in [VSCode Insiders](https://code.visualstudio.com/insiders/)
3. `npm install`
4. Run using VSCode

#### In Browser

1. `just wasm-web` (builds .wasm files and code bundle)
2. `cd lilypad-web`
7. `http-server -p 8000`
