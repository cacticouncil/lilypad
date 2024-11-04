# clang environment variables for web assembly
export AR := "llvm-ar"
export CFLAGS_wasm32_unknown_unknown := `echo "-I$(pwd)/wasm-sysroot -Wbad-function-cast -Wcast-function-type -fno-builtin"`

wasm-vscode:
    wasm-pack build --target web --release --no-typescript --out-dir lilypad-vscode/static

wasm-vscode-dev:
    wasm-pack build --target web --dev --no-typescript --out-dir lilypad-vscode/static

wasm-web:
    wasm-pack build --target web --release --no-typescript --out-dir lilypad-web/editor/src
    cd lilypad-web/editor && npm run build

wasm-web-dev:
    wasm-pack build --target web --dev --no-typescript --out-dir lilypad-web/editor/src
