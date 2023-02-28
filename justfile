wasm-vscode:
    wasm-pack build --target web --dev --no-typescript --out-dir lilypad-vscode/static

wasm-web:
    wasm-pack build --target web --dev --no-typescript
