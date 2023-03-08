import init, { run_editor, update_text, copy_selection, cut_selection, insert_text } from "./lilypad_web.js";

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
  vscode.postMessage({
    type: "edited",
    text: newText,
    range: range,
  });
}

export function setClipboard(text) {
  vscode.postMessage({
    type: "set_clipboard",
    text: text,
  });
}

// extension -> web view messages
window.addEventListener("message", event => {
  const message = event.data;
  switch (message.type) {
    case "update":
      update_text(message.text);
      break;
    case "copy":
      copy_selection();
      break;
    case "cut":
      cut_selection();
      break;
    case "paste":
      insert_text(message.text);
      break;
  }
});

run();
