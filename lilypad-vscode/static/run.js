import init, { run_editor, update_text } from "./lilypad_web.js";

async function run() {
  await init();
  run_editor();
}

// web view -> extension messages
const vscode = acquireVsCodeApi();

export function started() {
  vscode.postMessage({
    type: "started",
  });
}

export function edited(newText, startLine, startCol, endLine, endCol) {
  const range = { startLine, startCol, endLine, endCol };
  console.log(range);
  vscode.postMessage({
    type: "edited",
    text: newText,
    range: range,
  });
}

// extension -> web view messages
window.addEventListener('message', event => {
  const message = event.data;
  switch (message.type) {
    case "update":
        update_text(message.text);
        break;
  }
});

run();
