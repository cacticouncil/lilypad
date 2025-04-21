import * as vscode from "vscode";
import { DebugProtocol } from "@vscode/debugprotocol";
import { activeLilypadEditor, logger, setActiveLilypadEditor } from "./extension";

export class LilypadEditorProvider implements vscode.CustomTextEditorProvider {
    private internalEdit = false;

    private static readonly viewType = "lilypad.frameBased";

    public static register(context: vscode.ExtensionContext): vscode.Disposable {
        const provider = new LilypadEditorProvider(context);
        const options: vscode.WebviewPanelOptions = {
            retainContextWhenHidden: true,
        };
        const providerRegistration = vscode.window.registerCustomEditorProvider(
            LilypadEditorProvider.viewType,
            provider,
            { webviewOptions: options }
        );
        return providerRegistration;
    }

    constructor(
        private readonly context: vscode.ExtensionContext
    ) { }

    public async resolveCustomTextEditor(
        document: vscode.TextDocument,
        webviewPanel: vscode.WebviewPanel,
        token: vscode.CancellationToken
    ): Promise<void> {
        // Create Webview
        webviewPanel.webview.options = {
            enableScripts: true,
        };
        webviewPanel.webview.html = this.getHtml(webviewPanel.webview, document);

        // Sync our editor to external changes
        const changeDocumentSubscription = vscode.workspace.onDidChangeTextDocument(e => {
            if (e.document.uri.toString() === document.uri.toString()) {
                // sometimes there are random empty triggers,
                // don't change internal edits on those
                if (e.contentChanges.length === 0) {
                    return;
                }

                // don't notify of edit that the editor made itself
                // from what I can tell, edits are sent in order so this should work
                if (this.internalEdit) {
                    this.internalEdit = false;
                    return;
                }

                // notify
                for (const change of e.contentChanges) {
                    webviewPanel.webview.postMessage({
                        type: "apply_edit",
                        edit: change,
                    });
                }
            }
        });

        // Listen for new diagnostics
        const changeDiagnosticsSubscription = vscode.languages.onDidChangeDiagnostics(e => {
            if (e.uris.map(u => u.toString()).includes(document.uri.toString())) {
                webviewPanel.webview.postMessage({
                    type: "set_diagnostics",
                    diagnostics: vscode.languages.getDiagnostics(document.uri)
                });
            }
        });

        // Tracking which lilypad is active
        const viewStateSubscription = webviewPanel.onDidChangeViewState(e => {
            if (e.webviewPanel.active) {
                setActiveLilypadEditor(webviewPanel.webview);
            } else if (activeLilypadEditor === e.webviewPanel.webview) {
                setActiveLilypadEditor(null);
            }
        });

        // Tracking settings changes
        const configSubscription = vscode.workspace.onDidChangeConfiguration(e => {
            if (e.affectsConfiguration("lilypad.blocksTheme")) {
                const newTheme = vscode.workspace.getConfiguration("lilypad").get("blocksTheme");
                webviewPanel.webview.postMessage({
                    type: "set_blocks_theme",
                    theme: newTheme
                });
            } else if (e.affectsConfiguration("editor.fontFamily") || e.affectsConfiguration("editor.fontSize")) {
                // TODO: support fallback fonts instead of only sending the first
                // TODO: could this be called as a part of started instead of using the hacky js pass through thing?
                const editorConfig = vscode.workspace.getConfiguration("editor");
                const fontFamily = (editorConfig.get("fontFamily") as string).split(',')[0];
                const fontSize = editorConfig.get("fontSize");
                webviewPanel.webview.postMessage({
                    type: "set_font",
                    fontFamily,
                    fontSize
                });
            }
        });

        // Listen for new debug stuff
        function setBreakpoints() {
            let lines = [];
            for (let b of vscode.debug.breakpoints) {
                if (b instanceof vscode.SourceBreakpoint) {
                    lines.push(b.location.range.start.line);
                }
            }

            webviewPanel.webview.postMessage({
                type: "set_breakpoints",
                breakpoints: lines
            });
        }

        const changeBreakpointsSubscription = vscode.debug.onDidChangeBreakpoints(_e => {
            setBreakpoints();
        });

        function setStackFrame(activeStackItem: vscode.DebugStackFrame | vscode.DebugThread | undefined) {
            if (activeStackItem === undefined) { return; }

            vscode.debug.activeDebugSession!.customRequest('stackTrace', {
                threadId: activeStackItem.threadId
            }).then(response => {
                const stackFrames: Array<DebugProtocol.StackFrame> = response.stackFrames;

                let selectedFrameId = "frameId" in activeStackItem ? activeStackItem.frameId : undefined;
                let selectedFrame = stackFrames.find(s => s.id === selectedFrameId);
                let deepestFrame = stackFrames[0];

                let selectedFrameInFile = selectedFrame?.source?.path === document.uri.fsPath;
                let deepestFrameInFile = deepestFrame.source?.path === document.uri.fsPath;

                let selectedFrameLine = selectedFrameInFile ? selectedFrame?.line : undefined;
                let deepestFrameLine = deepestFrameInFile ? deepestFrame.line : undefined;

                webviewPanel.webview.postMessage({
                    type: "set_stack_frame",
                    selected: selectedFrameLine,
                    deepest: deepestFrameLine
                });
            });
        }

        const stackItemSubscription = vscode.debug.onDidChangeActiveStackItem(e => {
            setStackFrame(e);
        });

        vscode.debug.onDidTerminateDebugSession(e => {
            webviewPanel.webview.postMessage({
                type: "set_stack_frame",
                selected: undefined,
                deepest: undefined
            });
        });

        // Get rid of the listeners when our editor is closed.
        webviewPanel.onDidDispose(() => {
            changeDocumentSubscription.dispose();
            changeDiagnosticsSubscription.dispose();
            viewStateSubscription.dispose();
            configSubscription.dispose();
            changeBreakpointsSubscription.dispose();
            stackItemSubscription.dispose();
        });
        function convertHoverToEguiMarkdown(hoverResult) {
            if (!hoverResult) {
                return "";
            }
            //In vscode api, hover can be a markdown string, a markedstring(deprecated) or an
            //array of both of them, or an array of just one of them
            let markdownContent = "";
            if (Array.isArray(hoverResult.contents)) {
                for (const content of hoverResult.contents) {
                    if (typeof content === 'string') {
                        markdownContent += processMarkdownString(content) + "\n\n";
                    } else if (content instanceof vscode.MarkdownString) {
                        markdownContent += processMarkdownString(content.value) + "\n\n";
                    }
                }
            } else if (typeof hoverResult.contents === 'string') {
                markdownContent = processMarkdownString(hoverResult.contents);
            } else if (hoverResult.contents instanceof vscode.MarkdownString) {
                markdownContent = processMarkdownString(hoverResult.contents.value);
            }
            //yeah idk about this one but it works
            return markdownContent.trim();
        }
        //Make vscode extension's markdown string like egui commonmark
        function processMarkdownString(markdown) {
            if (!markdown) return "";
            
            let processed = markdown;
            
            //Code blocks always start with ``` followed by the language name(so far)
            processed = processed.replace(/```(\w+)/g, '```$1');

            //processed = processed.replace(/sharp/g, '#'); //Tried to see if replacing ```csharp 
            //with ```c# would fix the fact that csharp text not colored
            processed = processed.replace(/\[([^\]]+)\]\(command:[^\)]+\)/g, '$1');
            processed = processed.replace(/`([^`]+)`/g, '`$1`');
            
            //Remove ul and header in random python hover info is ##
            processed = processed.replace(/^(#+)([^\s#])/gm, '$1 $2');
            processed = processed.replace(/<\/?ul[\/]?>/g, '');

            //Get rid of the <!-- --> module hash in python hover infos
            processed = processed.replace(/^(\s*[-*+])([^\s])/gm, '$1 $2');
            processed = processed.replace(/<!--\s*.*?\s*-->/g, '');
            //get rid of run/debug in rust main function hover(there is a little button in the hover info on normal vscode that you can click run but)
            processed = processed.replace(/▶︎ Run  | ⚙︎ Debug/g, '')
            return processed;
        }


        // Receive message from the webview.
        webviewPanel.webview.onDidReceiveMessage(message => {
            switch (message.type) {
                case "started": {
                    // give initial text
                    webviewPanel.webview.postMessage({
                        type: "set_text",
                        text: document.getText(),
                    });

                    // send initial diagnostics
                    webviewPanel.webview.postMessage({
                        type: "set_diagnostics",
                        diagnostics: vscode.languages.getDiagnostics(document.uri)
                    });

                    // send initial breakpoints
                    setBreakpoints();

                    // send initial stack frame
                    setStackFrame(vscode.debug.activeStackItem);

                    // set the new webview as the current webview
                    setActiveLilypadEditor(webviewPanel.webview);
                    break;
                }
                case "edited": {
                    const editedRange = new vscode.Range(
                        message.range.startLine,
                        message.range.startCol,
                        message.range.endLine,
                        message.range.endCol
                    );
                    this.updateTextDocument(document, message.text, editedRange);
                    break;
                }
                case "set_clipboard": {
                    vscode.env.clipboard.writeText(message.text);
                    break;
                }
                case "get_quick_fixes": {
                    const cursor = new vscode.Range(message.line, message.col,
                        message.line, message.col);
                    vscode.commands.executeCommand<vscode.CodeAction[]>(
                        "vscode.executeCodeActionProvider",
                        document.uri,
                        cursor,
                        vscode.CodeActionKind.QuickFix.value
                    ).then((actions) => {
                        webviewPanel.webview.postMessage({
                            type: "return_quick_fixes",
                            id: message.id,
                            actions
                        });
                    });
                    break;
                }
                case "get_completions": {
                    const cursor = new vscode.Position(message.line, message.col);
                    vscode.commands.executeCommand<vscode.CompletionList>(
                        "vscode.executeCompletionItemProvider",
                        document.uri,
                        cursor
                    ).then((completions) => {
                        webviewPanel.webview.postMessage({
                            type: "return_completions",
                            completions: completions.items
                        });
                    });
                    break;
                }
                case "get_hover": {
                    const URI = document.uri; 
                    const cursor = new vscode.Position(message.line, message.col);
                    vscode.commands.executeCommand<vscode.Hover[]>(
                        'vscode.executeHoverProvider', 
                        URI, 
                        cursor
                    ).then((hover) => {
                        if (hover && hover[0]) {
                            //if (hover[0].)
                           // console.log("Hover info:", hover.length);
                            let hoverToConvert = hover[0];
                            if (hover.length > 1) {
                                hoverToConvert = hover[1];
                            }
                            const hoverContent = convertHoverToEguiMarkdown(hoverToConvert);
                            //console.log("Hover content:", hoverContent);
                        
                            
                            webviewPanel.webview.postMessage({
                                type: "return_hover_info",
                                hover: hoverContent,
                                range: hover[0].range
                            });
                        
                        } else {
                            //No hover info
                        }
                    });
                    break;
                }
                case "register_breakpoints": {
                    // currently just reset all breakpoints, could change this to a diff later if that matters
                    vscode.debug.removeBreakpoints(vscode.debug.breakpoints);
                    const newBreakpoints: Array<vscode.SourceBreakpoint> = Array.from(message.lines, ((line: number) =>
                        new vscode.SourceBreakpoint(new vscode.Location(document.uri, new vscode.Position(line, 0)))
                    ));
                    vscode.debug.addBreakpoints(newBreakpoints);
                    break;
                }
                case "execute_command": {
                    vscode.commands.executeCommand(message.command, ...message.args);
                    break;
                }
                case "execute_workspace_edit": {
                    let edit = message.edit as vscode.WorkspaceEdit | null;
                    if (edit) {
                        vscode.workspace.applyEdit(edit);
                    }
                    break;
                }
                case "telemetry_log": {
                    logger.logUsage(message.cat, message.info);
                    break;
                }
                case "telemetry_crash": {
                    logger.logError(new Error(message.msg));

                    // reload the page if crashed
                    vscode.commands.executeCommand("workbench.action.webview.reloadWebviewAction");

                    break;
                }
            }
        });
    }

    private editQueue: vscode.WorkspaceEdit[] = [];
    private applyingEdit = false;

    private updateTextDocument(document: vscode.TextDocument, newText: string, range: vscode.Range) {
        const edit = new vscode.WorkspaceEdit();
        edit.replace(
            document.uri,
            range,
            newText
        );
        this.editQueue.push(edit);
        this.applyQueueEdit();
    }

    private applyQueueEdit() {
        // do nothing if already applying edits
        if (this.applyingEdit) {
            return;
        }

        // apply first element in queue, if it exists
        let nextEdit = this.editQueue.shift();
        if (nextEdit) {
            // don't allow another edit to be applied until this finishes
            this.applyingEdit = true;

            // don't cause edit notification cycle
            this.internalEdit = true;

            // apply edit, and then trigger the next edit after that
            vscode.workspace.applyEdit(nextEdit)
                .then(res => {
                    if (res) {
                        this.applyingEdit = false;
                        this.applyQueueEdit();
                    } else {
                        console.error("Failed to apply edit");
                    }
                });
        }
    }

    private getHtml(webview: vscode.Webview, document: vscode.TextDocument): string {
        const scriptUri = webview.asWebviewUri(vscode.Uri.joinPath(
            this.context.extensionUri, "static", "run.js"));

        // get the font settings for vscode
        // TODO: support fallback fonts instead of only sending the first
        const editorConfig = vscode.workspace.getConfiguration("editor");
        const fontFamily = (editorConfig.get("fontFamily") as string).split(',')[0];
        const fontSize = editorConfig.get("fontSize");

        // get the block theme
        const blockTheme = vscode.workspace.getConfiguration("lilypad").get("blocksTheme");

        return `
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <title>Lilypad Editor</title>
                <style>
                    html,
                    body {
                        overflow: hidden;
                        margin: 0 !important;
                        padding: 0 !important;
                        height: 100%;
                        width: 100%;
                    }

                    canvas {
                        margin-right: auto;
                        margin-left: auto;
                        display: block;
                        position: absolute;
                        width: 100%;
                        height: 100%;
                    }
                </style>
            </head>
            <body>
                <div style="text-align: center; margin: 0px; padding: 0px;">
                    <canvas id="lilypad-canvas"></canvas>
                </div>
                <script>
                    /* this is a hacky way to send the configuration to run.js */
                    var fileName = "${document.fileName}"
                    var fontFamily = "${fontFamily}"
                    var fontSize = ${fontSize}
                    var blockTheme = "${blockTheme}"
                </script>
                <script
                    type="module"
                    src="${scriptUri}"
                    type="application/javascript"
                ></script>
            </body>
        </html>
        `;
    }
}
