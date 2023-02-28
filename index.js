import init, { wasm_main } from "./pkg/lilypad_web.js";

async function run() {
  await init();
  wasm_main();
}

run();
