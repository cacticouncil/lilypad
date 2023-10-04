import init, { run_editor, set_file } from "./lilypad_web.js";

async function run() {
  await init();
  run_editor("test.py", "Roboto Mono", 14);
}

// Functions that the editor calls to communicate
// TODO: wire these up
export function started() {
  // send a resize event to the window to make sure the editor is sized correctly
  // run after a delay so things have the oppurtunity to appear
  setTimeout(() => {
    window.dispatchEvent(new UIEvent("resize"));
  }, 10);
}
export function edited(newText, startLine, startCol, endLine, endCol) { }
export function setClipboard(text) { }
export function requestQuickFixes(line, col) { }
export function executeCommand(command, args) { }
export function requestCompletions(line, col) { }
export function executeWorkspaceEdit(edit) { }

document.getElementById("language-picker").addEventListener("change", (e) => {
  const language = e.target.value;
  set_file("test." + language);
});

// start after the window loads so is calculates the font size after it loads it
window.onload = run;
