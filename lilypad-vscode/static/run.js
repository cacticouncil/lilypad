import init, { LilypadWebHandle } from "./lilypad_web.js";

await init();
const handle = new LilypadWebHandle();
const vscode = acquireVsCodeApi();

// fileName, fontFamily, fontSize, and blockTheme are set in another script tag
await handle.start("lilypad-canvas", fileName, fontFamily, fontSize, blockTheme);

/* --------------------- web view -> extension messages --------------------- */

export function started() {
  // send a resize event to the window to make sure the editor is sized correctly
  // run after a delay so things have the oppurtunity to appear
  setTimeout(() => {
    window.dispatchEvent(new UIEvent("resize"));
  }, 50);

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

export function requestQuickFixes(id, line, col) {
  vscode.postMessage({
    type: "get_quick_fixes",
    id: id,
    line: line,
    col: col,
  });
}

export function requestCompletions(line, col) {
  vscode.postMessage({
    type: "get_completions",
    line: line,
    col: col,
  });
}

export function executeCommand(command, args) {
  vscode.postMessage({
    type: "execute_command",
    command: command,
    args: args,
  });
}

export function executeWorkspaceEdit(edit) {
  vscode.postMessage({
    type: "execute_workspace_edit",
    edit: edit
  });
}

export function registerBreakpoints(lines) {
  vscode.postMessage({
    type: "register_breakpoints",
    lines: lines
  });
}

export function telemetryEvent(cat, info) {
  vscode.postMessage({
    type: "telemetry_log",
    cat: cat,
    info: Object.fromEntries(info)
  });
}

export function telemetryCrash(msg) {
  vscode.postMessage({
    type: "telemetry_crash",
    msg: msg,
  });
}

/* --------------------- extension -> web view messages --------------------- */
window.addEventListener("message", event => {
  const message = event.data;
  switch (message.type) {
    case "set_text":
      handle.set_text(message.text);
      break;
    case "apply_edit":
      handle.apply_edit(message.edit);
      break;
    case "set_diagnostics":
      handle.set_diagnostics(message.diagnostics);
      break;
    case "set_blocks_theme":
      handle.set_blocks_theme(message.theme);
      break;
    case "set_breakpoints":
      handle.set_breakpoints(message.breakpoints);
      break;
    case "set_stack_frame":
      handle.set_stack_frame(message.selected, message.deepest);
      break;
    case "return_quick_fixes":
      handle.set_quick_fixes(message.id, message.actions);
      break;
    case "return_completions":
      handle.set_completions(message.completions);
      break;
    case "undo":
      handle.undo();
      break;
    case "redo":
      handle.redo();
      break;
    default:
      console.error("Unknown message type: " + message.type);
  }
});

/* ------------------------ handle clipboard actions ------------------------ */
document.addEventListener("copy", function (e) {
  copy_selection();
  e.preventDefault();
});

document.addEventListener("cut", function (e) {
  cut_selection();
  e.preventDefault();
});

document.addEventListener("paste", function (e) {
  let text = e.clipboardData.getData("text/plain");
  insert_text(text);
  e.preventDefault();
});
