import init, { run_editor } from "./lilypad_web.js";

async function run() {
  await init();
  run_editor("Roboto Mono", 14);
}

// Functions that the editor calls to communicate
// TODO: wire these up
export function started() { }
export function edited(newText, startLine, startCol, endLine, endCol) { }
export function setClipboard(text) { }
export function requestQuickFixes(line, col) { }
export function executeCommand(command, args) { }

run();
