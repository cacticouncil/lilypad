wasm-vscode:
    wasm-pack build --target web --release --no-typescript --out-dir lilypad-vscode/static

wasm-vscode-dev:
    wasm-pack build --target web --dev --no-typescript --out-dir lilypad-vscode/static

wasm-web:
    wasm-pack build --target web --release --no-typescript --out-dir lilypad-web/editor

wasm-web-dev:
    wasm-pack build --target web --dev --no-typescript --out-dir lilypad-web/editor
