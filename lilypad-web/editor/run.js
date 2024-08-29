import init, { LilypadWebHandle } from "./lilypad_web.js";

await init();
const handle = new LilypadWebHandle();
await handle.start("lilypad-canvas", "test.py", "Roboto Mono", 14, "syntax_colored");

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
export function setClipboard(text) {
  navigator.clipboard.writeText(text);
}
export function requestQuickFixes(line, col) { }
export function executeCommand(command, args) { }
export function requestCompletions(line, col) { }
export function executeWorkspaceEdit(edit) { }
export function registerBreakpoints(lines) { }
export function telemetryEvent(cat, info) { }
export function telemetryCrash(msg) { }

// handle clipboard actions
document.addEventListener("copy", function (e) {
  handle.copy_selection();
  e.preventDefault();
});

document.addEventListener("cut", function (e) {
  handle.cut_selection();
  e.preventDefault();
});

addEventListener("paste", (event) => {
  handle.insert_text(event.clipboardData.getData("text"));
});

document.getElementById("language-picker").addEventListener("change", (e) => {
  const language = e.target.value;
  set_file("test." + language);
});

// start running after the font is downloaded so it can be measured at launch
// const robotoMono = new FontFace(
//   "Roboto Mono",
//   "url(https://fonts.gstatic.com/s/robotomono/v23/L0xuDF4xlVMF-BfR8bXMIhJHg45mwgGEFl0_3vq_ROW-AJi8SJQt.woff)",
// );
// document.fonts.add(robotoMono);
// document.fonts.load("14pt Roboto Mono").then((_) => {
//   run();
// })
