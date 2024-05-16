import init, { run_editor, set_text, apply_edit, copy_selection, cut_selection, insert_text, new_diagnostics, set_quick_fixes, set_completions, set_block_theme, undo, redo, set_breakpoints, set_stack_frame } from "./lilypad_web.js";

async function run() {
  await init();
  // fileName, fontFamily, fontSize, and blockTheme are set in another script tag
  run_editor(fileName, fontFamily, fontSize, blockTheme);
}
// web view -> extension messages
const vscode = acquireVsCodeApi();

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

export function requestQuickFixes(line, col) {
  vscode.postMessage({
    type: "get_quick_fixes",
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

// extension -> web view messages
window.addEventListener("message", event => {
  const message = event.data;
  switch (message.type) {
    case "set_text":
      set_text(message.text);
      break;
    case "apply_edit":
      apply_edit(message.edit);
      break;
    case "new_diagnostics":
      new_diagnostics(message.diagnostics);
      break;
    case "new_blocks_theme":
      set_block_theme(message.theme);
      break;
    case "set_breakpoints":
      set_breakpoints(message.breakpoints);
      break;
    case "set_stack_frame":
      set_stack_frame(message.selected, message.deepest);
      break;
    case "return_quick_fixes":
      set_quick_fixes(message.actions);
      break;
    case "return_completions":
      set_completions(message.completions);
      break;
    case "undo":
      undo();
      break;
    case "redo":
      redo();
      break;
  }
});

// handle clipboard actions
document.addEventListener("copy", function(e) {
  copy_selection();
  e.preventDefault();
});

document.addEventListener("cut", function(e) {
  cut_selection();
  e.preventDefault();
});

document.addEventListener("paste", function(e) {
  let text = e.clipboardData.getData("text/plain");
  insert_text(text);
  e.preventDefault();
});

// start the editor
run();
