import * as vscode from "vscode";
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
                    type: "new_diagnostics",
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
                    type: "new_blocks_theme",
                    theme: newTheme
                });
            }
        });

        // Get rid of the listeners when our editor is closed.
        webviewPanel.onDidDispose(() => {
            changeDocumentSubscription.dispose();
            changeDiagnosticsSubscription.dispose();
            viewStateSubscription.dispose();
            configSubscription.dispose();
        });

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
                        type: "new_diagnostics",
                        diagnostics: vscode.languages.getDiagnostics(document.uri)
                    });

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
                <title>Druid web example</title>
                <style>
                    html,
                    body,
                    canvas {
                        margin: 0px;
                        padding: 0px;
                        width: 100%;
                        height: 100%;
                        overflow: hidden;
                    }
                </style>
            </head>
            <body>
                <div style="text-align: center; margin: 0px; padding: 0px;">
                    <canvas id="canvas"></canvas>
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
